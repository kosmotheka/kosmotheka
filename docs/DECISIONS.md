# Registro de decisiones

> Estados: `abierta` · `propuesta` · `aceptada` · `reemplazada por Dxx`. Ver convenciones en [README.md](README.md).

## Resumen

| # | Decisión | Estado |
|---|---|---|
| D01 | Idioma del repositorio | aceptada |
| D02 | Estilo de código y comentarios | aceptada |
| D03 | Repositorio privado ahora, público después | aceptada |
| D04 | Todo open source | aceptada |
| D05 | Arquitectura local-first con servidor autoritativo y motor de sincronización propio | aceptada |
| D06 | Servidor en Symfony | aceptada |
| D07 | SQLite en el cliente, PostgreSQL en el servidor | aceptada |
| D08 | Catálogos con herencia por tablas | aceptada |
| D09 | Autenticación simple | aceptada |
| D10 | Alojamiento en VPS | aceptada |
| D11 | Licencia AGPL para todo; especificación con licencia permisiva | aceptada |
| D12 | Documentación continua | aceptada |
| D13 | Sincronización local entre dispositivos con emparejamiento por QR | aceptada |
| D14 | Acceso a SQLite desde Rust: `rusqlite` | aceptada |
| D15 | Interfaz: React + TypeScript + Vite + shadcn/ui | aceptada |
| D16 | Monorepo | aceptada |
| D17 | Aplicación multidioma desde el primer día | aceptada |
| D18 | Servicios externos siempre opcionales | aceptada |
| D19 | Carga masiva y reconocimiento de estanterías | aceptada |
| D20 | IA de varios proveedores para reconocer imágenes | abierta (futuro) |
| D21 | Proveedores de metadatos por tipo de catálogo | propuesta |
| D22 | Web pública y sitio de documentación | abierta (lejano) |
| D23 | Entorno de desarrollo en Linux: Distrobox | aceptada |
| D24 | Tres niveles: obra, edición y ejemplar | aceptada |
| D25 | Operaciones: cambios de entidad con etiqueta de intención | aceptada |
| D26 | Búsqueda por ISBN: Open Library + Google Books con clave opcional | propuesta |
| D27 | Portadas: ficheros locales con nombre por hash | propuesta |
| D28 | Alta rápida: guarda lo encontrado, no duplica lo que ya existe | propuesta |
| D29 | Editoriales como catálogo y búsqueda con FTS5 | propuesta |
| D30 | Copias de seguridad (ZIP) y CSV | propuesta |
| D31 | Operaciones replicables, versión de esquema y operación inicial | propuesta |
| D32 | Aplicar operaciones recibidas: campo a campo y el borrado gana | propuesta |
| D33 | Duplicados entre dispositivos: se conservan y se fusionan a mano | propuesta |
| D34 | Transporte local: HTTPS con certificado fijado por el QR | propuesta |
| D35 | Escáner de códigos con el plugin oficial de Tauri (solo móvil) | propuesta |
| D36 | Lista de deseos básica, sincronizada | propuesta |

---

## D01 · Idioma del repositorio — `aceptada`

Todo el repositorio en **inglés**: código, identificadores, comentarios, mensajes de commit, README raíz, issues. La carpeta `docs/` va en **español** mientras construimos, para trabajar más cómodos; se traducirá antes de publicar.

## D02 · Estilo de código y comentarios — `aceptada`

Código limpio y legible. Comentarios en inglés, **justos**: explican el porqué cuando no es obvio, no repiten lo que el código ya dice.

## D03 · Repositorio privado ahora, público después — `aceptada`

Repositorio privado en GitHub durante la construcción, con la intención de abrirlo. Consecuencia práctica: desde el primer commit, **nada de secretos en el repo** (claves de API, contraseñas) y nada que dé vergüenza publicar. Así abrirlo será cambiar un interruptor, no hacer limpieza del historial.

## D04 · Todo open source — `aceptada`

La app y el servidor se publicarán como open source. Cualquiera podrá autoalojar su servidor, incluidas las asociaciones y ludotecas; el servicio alojado será el «oficial».

Motivos:
- En un servicio gratuito, lo que tiene valor es **mantenerlo funcionando** (copias de seguridad, disponibilidad, actualizaciones), no el código.
- Cerrar el servidor impediría el autoalojamiento.
- Para evitar que alguien monte un servicio cerrado con el código basta la licencia (D11).

Opciones descartadas: publicar solo la app, con el servidor privado, y el modelo *open core*.

## D05 · Arquitectura local-first con servidor autoritativo y motor de sincronización propio — `aceptada`

- La app **siempre** lee y escribe en su base de datos local (SQLite). La interfaz nunca espera a la red.
- **Modo local** (sin cuenta): la app funciona sola. Es un producto completo para quien quiere su catálogo privado.
- **Modo conectado** (con cuenta en un servidor): la app sigue escribiendo en local, pero además envía cada cambio al servidor como una **operación** («añadir ejemplar», «borrar edición»…). El servidor **vuelve a aplicar la operación con sus propias reglas**: si el usuario no tiene permiso, la rechaza, y el cliente deshace el cambio local. El servidor es la fuente de la verdad; el cliente es una réplica que puede adelantarse.

Es el patrón de mutaciones optimistas que el servidor vuelve a aplicar con autoridad (Linear, Replicache). Resuelve el problema de permisos que hizo descartar el local-first en la ideación.

**Motor de sincronización propio**, no una librería externa: el objetivo es aprender los conceptos. Antes de diseñarlo hemos revisado los motores existentes y las ideas que tomamos de ellos (ver [research/sync-engines.md](research/sync-engines.md)).

**Consecuencias.**
- El modo sin conexión forma parte de la arquitectura desde el primer día; no es una caché que se añade al final.
- El modelo de datos funciona igual en SQLite y en PostgreSQL (D08).
- Los identificadores se generan en el cliente (UUID v7), no son autoincrementales.
- Las operaciones son **independientes del transporte**: el mismo formato sirve para sincronizar con el servidor (HTTP) y entre dispositivos en la red local (D13).

## D06 · Servidor en Symfony — `aceptada`

Symfony es el stack que dominas y con el que disfrutas. Las barreras que pone a quien quiera contribuir (PHP, curva de aprendizaje de Symfony) se compensan con una buena imagen Docker, buena documentación y un código claro.

Node se descartó porque no es «más abierto» que Symfony, que es software libre (licencia MIT), y porque no lo dominas. Rust (Axum) se descartó porque aprender a la vez el framework web y Tauri es demasiado; además, disfrutar del proyecto pesa más que no duplicar código.

**Consecuencia y cómo mitigarla.** Las reglas de las operaciones (qué es una operación válida y qué efecto tiene) existirán dos veces: en Rust, en el cliente, y en PHP, en el servidor. Para que no se desincronicen:
- **Especificación común de las operaciones** en un formato neutro (JSON Schema) dentro del repo. Ninguno de los dos lenguajes es la referencia: lo es la especificación.
- **Pruebas de conformidad compartidas**: una carpeta de casos en JSON («estado inicial + operación → estado final esperado o rechazo») que ejecutan tanto los tests de Rust como los de PHP. Si una implementación diverge, falla la CI.

## D07 · SQLite en el cliente, PostgreSQL en el servidor — `aceptada`

- **Cliente:** SQLite (lo impone el local-first y es lo estándar en Tauri).
- **Servidor:** PostgreSQL.
- **Descartado: una base de datos NoSQL.** Los datos son muy relacionales: ediciones, ejemplares, usuarios, préstamos y lecturas se cruzan constantemente.

## D08 · Catálogos con herencia por tablas — `aceptada`

- Una tabla común `items` con lo compartido (título, portada, enlaces externos). Ejemplares, préstamos y lecturas apuntan siempre a `items`.
- Una tabla 1:1 por tipo, con columnas normales: `book_details`, `board_game_details`…
- Una columna JSON opcional `extra`, solo para guardar la respuesta en bruto de fuentes externas (Google Books, BoardGameGeek) que no se consulta. Si un campo empieza a consultarse, se convierte en columna.
- Añadir un tipo de catálogo supone una migración; se aceptan porque habrá pocos tipos y se añadirán muy de vez en cuando.

Se descartaron: tabla común + JSONB (consultas incómodas y mala portabilidad entre SQLite y PostgreSQL) y tablas totalmente separadas por tipo (duplican lo común).

## D09 · Autenticación simple — `aceptada`

Usuario y contraseña, sin diseñar aún el multiusuario público ni el login social. Solo aplica al modo conectado. La app necesitará un token de larga duración para sincronizar, en lugar de una cookie de sesión web. Se revisará cuando llegue la versión pública.

## D10 · Alojamiento en VPS — `aceptada`

No es urgente hasta la fase del servidor.

## D11 · Licencia AGPL para todo; especificación con licencia permisiva — `aceptada`

- **App y servidor: AGPL-3.0.** Quien modifique el código y lo distribuya, o lo ofrezca como servicio, tiene que publicar sus cambios. Es coherente con D04 y fácil de explicar: todo lo que se construya sobre esto sigue abierto.
- **Especificación de operaciones (`spec/`): MIT.** Así cualquiera puede escribir un cliente o un servidor compatible sin heredar la AGPL. Esto fomenta un ecosistema sin abrir la puerta a que alguien cierre nuestro código.

**Por qué no Apache o MIT para todo.** Permitirían que alguien publicara una versión cerrada de la app o del servidor. No es grave (Koillection es MIT y le va bien), pero choca con el espíritu de D04.

**Matices a tener en cuenta.**
- Mientras seas el único autor puedes cambiar la licencia cuando quieras. Cuando lleguen contribuciones externas, cambiarla requerirá el permiso de cada colaborador. Se puede resolver más adelante con un acuerdo de contribución (CLA) o con una firma de origen (DCO), si hace falta.
- La App Store de iOS es incompatible con las licencias GPL y AGPL cuando la app incluye código de terceros bajo esas licencias. Como titular puedes publicarla tú, pero el problema aparece con las contribuciones externas. Hoy no afecta: solo hay Android por APK, e iOS no está en el plan.

## D12 · Documentación continua — `aceptada`

Se documenta a medida que se construye (ver [README.md](README.md)). Cada fase del roadmap entrega, además del código, su documentación.

## D13 · Sincronización local entre dispositivos con emparejamiento por QR — `aceptada`

**Contexto.** Antes de que exista el servidor, los libros tienen que poder pasar del escritorio al móvil. Exportar e importar un fichero es un rollo. Principio: **para el usuario, siempre muy fácil**.

**Por qué el QR no transporta los datos.** Un código QR solo cabe unos 3 KB. Da para unas pocas decenas de libros sin portadas, no para un catálogo. El QR sirve para **emparejar**; los datos viajan por la **red local**. Es el patrón de apps como KDE Connect o LocalSend.

**Flujo propuesto.**
1. El escritorio muestra un QR con su dirección en la red local, un puerto y la huella de su clave pública.
2. El móvil lo escanea con la misma cámara que usa para los ISBN.
3. El móvil se conecta por wifi al escritorio y comprueba la clave contra la huella del QR. Así la conexión queda cifrada y autenticada sin contraseñas ni certificados de una autoridad.
4. Cada lado envía al otro las operaciones que le faltan desde la última sincronización, portadas incluidas.
5. Los dispositivos quedan emparejados: a partir de entonces se sincronizan con un botón, o solos cuando coinciden en la red.

**El problema de fondo: no hay árbitro.** En modo local no hay servidor que ordene las operaciones, así que cada dispositivo debe llegar al mismo resultado por su cuenta. Esto obliga a aprender:
- **Relojes lógicos híbridos (HLC)**: cada operación lleva una marca de tiempo que las ordena igual en todos los dispositivos, aunque sus relojes no coincidan.
- **El último en escribir gana, campo a campo**: si en un dispositivo cambias la ubicación y en otro la valoración, se conservan los dos cambios.
- **Lápidas (*tombstones*)**: un borrado se registra como una operación más, para que el otro dispositivo no «resucite» el elemento.

Es la parte donde más se aprende del local-first, y también la más difícil.

**Paso intermedio para reducir riesgo.**
- **v1:** copia completa en un solo sentido, del escritorio al móvil. El móvil es una réplica de consulta, que basta para el «¿lo tengo ya?».
- **v2:** sincronización en los dos sentidos, por operaciones.

**Requisitos y fricciones.**
- Los dos dispositivos deben estar en la misma wifi.
- El cortafuegos de Windows preguntará la primera vez.
- Algunas redes, como las de invitados o las del trabajo, aíslan los dispositivos entre sí.

**Sincronización automática al llegar a casa.** Los dispositivos emparejados se anuncian y se descubren en la red local mediante mDNS/DNS-SD (Bonjour), así que no hace falta volver a escanear el QR. Limitación de Android: el sistema restringe mucho lo que una app hace en segundo plano. Lo realista es sincronizar al abrir la app en la wifi de casa, y probar después una sincronización periódica en segundo plano.

**Dispositivo principal (futuro).** En una familia en modo local se puede designar un dispositivo principal, normalmente el ordenador de casa, que hace el papel del servidor:
- Es el **árbitro**: ordena las operaciones y aplica los permisos. Así el hijo no puede borrar, igual que en el modo conectado.
- Las operaciones no permitidas no se pierden: se convierten en **solicitudes** («mi hijo pide borrar este libro») que un administrador aprueba o rechaza. Esta funcionalidad sirve también en el modo conectado.
- Consecuencia: la lógica de autoridad existirá en Rust (dispositivo principal) y en PHP (servidor). Es un motivo más para las pruebas de conformidad compartidas (D06).

**Consecuencias.**
- El formato de operaciones de D05 debe incluir desde el principio el identificador del dispositivo y la marca HLC.
- El emparejamiento por QR se podrá reutilizar después para **invitar a familiares al servidor**: un QR con la URL del servidor y un token de invitación.

## D14 · Acceso a SQLite desde Rust: `rusqlite` — `aceptada`

| Opción | Qué es | Valoración |
|---|---|---|
| **`rusqlite`** | Envoltorio directo y síncrono de SQLite. Con la opción `bundled` compila SQLite dentro de la app. | Simple, maduro y sin sorpresas. `bundled` garantiza la misma versión de SQLite en Linux, Windows y Android. El SQL es SQL de verdad, que ya dominas. Es el mejor para aprender Rust: no añade asincronía ni macros complejas. |
| `sqlx` | Acceso asíncrono, con consultas comprobadas en compilación contra una base de datos real. | Potente, pero la asincronía añade una capa difícil de Rust mientras aprendes, y la comprobación en compilación complica el flujo de desarrollo. Su ventaja real está en servidores con muchas conexiones, que no es nuestro caso. |
| `tauri-plugin-sql` | Plugin oficial de Tauri que permite lanzar SQL **desde el JavaScript** de la interfaz. | Descartado: pondría la lógica de datos en la interfaz, y en D05 decidimos que el dominio vive en Rust. |
| Diesel / SeaORM | ORM de Rust. | Descartados por ahora: una capa más que aprender, y el modelo lo definen las operaciones, no un ORM. |

**Propuesta:** `rusqlite` con `bundled`, más `rusqlite_migration` para las migraciones (ficheros SQL numerados, que se aplican al arrancar la app). Las migraciones del cliente y del servidor serán distintas (SQLite y Doctrine), pero ambas siguen el modelo de D08.

## D15 · Interfaz: React + TypeScript + Vite + shadcn/ui — `aceptada`

- Tauri no trae una librería de interfaz propia: es una ventana con un navegador del sistema dentro. Cualquier framework web sirve, y la plantilla oficial de React + TypeScript + Vite es la más documentada.
- shadcn/ui (sobre Tailwind y Radix) funciona sin problemas dentro de Tauri.
- **A tener en cuenta:** Tauri usa el navegador **del sistema**, no uno propio: WebView2 (Chromium) en Windows, WebKitGTK en Linux y el System WebView en Android. WebKitGTK a veces va por detrás en CSS y en algunas API web, así que hay que probar en los tres desde el principio.

## D16 · Monorepo — `aceptada`

Un único repositorio con la app, el servidor, el dominio en Rust y la especificación. La estructura concreta se fija al arrancar la Fase 1 (ver ROADMAP).

## D17 · Aplicación multidioma desde el primer día — `aceptada`

La interfaz nace con **inglés y español** y todos los textos en ficheros de traducción, nunca escritos en el código. Hacerlo al principio es trivial; hacerlo después es buscar cadenas por todo el código. Koillection lo hizo así y recibe traducciones de la comunidad en Crowdin. Librería candidata en React: `react-i18next`.

## D18 · Servicios externos siempre opcionales — `aceptada`

«Local-first» no significa «sin internet nunca». Buscar un ISBN en Google Books o en Open Library ya requiere conexión. El principio es otro:
- **Todo lo que ya está en tu catálogo funciona sin conexión y sin cuentas**: consultar, buscar, editar, prestar, sincronizar por la red local.
- **Los servicios externos enriquecen, pero nunca son imprescindibles.** Si no hay red o falla el servicio, se puede seguir con el alta manual y completarla después.
- **Ninguna clave de API es obligatoria.** Si un servicio necesita clave o tiene coste (como una IA en la nube), es opcional y el usuario lo activa expresamente. Puede usar su propia clave o, en modo conectado, el servidor puede ofrecerlo.
- Cada servicio externo va detrás de una **interfaz intercambiable** («proveedor de metadatos», «reconocedor de imágenes»). Esto facilita añadir fuentes nuevas (BoardGameGeek, scrapers) y probar con dobles en los tests.

## D19 · Carga masiva y reconocimiento de estanterías — `aceptada`

**Contexto.** La carga inicial de cientos de libros es imprescindible, y la foto de la estantería es la forma más atractiva de hacerla. Pero se quiere evitar depender de una API de IA: sería otra capa, con coste, claves y privacidad (D18).

**Las cuatro vías, de más fiable a más ambiciosa.**

| Vía | Cómo funciona | ¿Local? | Fiabilidad |
|---|---|---|---|
| **1. Escáner en ráfaga** | Cámara siempre activa: pasas los libros uno tras otro por el código de barras y la app los añade a una cola de revisión sin tocar la pantalla. Usa el plugin oficial de Tauri para escanear códigos de barras en móvil. | Sí (la búsqueda de metadatos usa internet, pero se puede diferir) | Muy alta. Unos 3 s por libro: 300 libros en unos 20 min |
| **2. Lector de códigos USB** | En escritorio, un lector de 20–30 € se comporta como un teclado: «escribe» el ISBN y pulsa Intro. | Sí | Muy alta, y casi sin desarrollo |
| **3. Foto + OCR local** | Se leen los lomos de la foto con un OCR en el dispositivo (ML Kit en Android; Tesseract u `ocrs`, un OCR escrito en Rust, en escritorio). Los fragmentos de texto se comparan de forma aproximada (*fuzzy*) con una lista de títulos. | Sí | **Depende de contra qué se compare** (ver abajo) |
| **4. Foto + IA en la nube** | Se envía la foto a un modelo de visión (Claude) que devuelve títulos y autores. | No | La más alta con fotos reales, pero con coste, clave y privacidad |

**La clave de la vía 3: el espacio de búsqueda.**
- **Inventario (comparar con tu propio catálogo): funciona bien en local.** Buscas entre unos pocos miles de títulos que ya conoces. Aunque el OCR lea «SEÑOR DE LOS AN», la comparación aproximada encuentra el libro sin dudar. Así el modo inventario puede usar fotos de la balda, no solo escaneo uno a uno.
- **Carga inicial (libros desconocidos): el OCR local se queda corto.** Los lomos tienen texto vertical, tipografías decorativas y reflejos, y el espacio de búsqueda es «todos los libros del mundo». Los fragmentos del OCR se pueden buscar en Open Library o en Google Books y el usuario confirma, pero espero bastante trabajo manual. Es una estimación que hay que validar.
- Existen modelos de visión pequeños que se ejecutan en local (con llama.cpp u ONNX). Son una capa todavía más pesada: descargas de gigas, lentos sin GPU e inviables en móvil. Se descartan por ahora.

**Propuesta.**
1. **La carga inicial se resuelve con las vías 1 y 2**, que son locales, gratuitas y fiables. Es el «imprescindible» real.
2. **Foto de balda para el inventario** con OCR local contra el propio catálogo (vía 3).
3. **Foto de balda para la carga inicial**: primero una **prueba de concepto** con fotos reales de tus estanterías, OCR local más búsqueda en Open Library, midiendo cuántos lomos acierta. Si no basta, la vía 4 queda como **extra opcional** detrás del «reconocedor de imágenes» de D18: el usuario pone su clave o, en modo conectado, el servidor la ofrece.
4. En cualquier vía, lo detectado va a una **cola de revisión**: nunca se da de alta nada sin confirmación.

**Matiz sobre el lector USB (vía 2).** Es opcional y casi gratis de soportar. Un lector USB se comporta como un teclado: escribe los dígitos del ISBN muy rápido y pulsa Intro. Basta con que el campo de ISBN acepte «número + Intro» y, en el modo ráfaga del escritorio, lance la búsqueda y vuelva a quedar listo. **Se puede probar sin tener un lector**, tecleando o pegando un ISBN y pulsando Intro. Es especialmente útil para bibliotecas y ludotecas.

## D20 · IA de varios proveedores para reconocer imágenes — `abierta (futuro)`

La vía 4 de D19 es interesante: tú la usarías, y quien quiera puede poner su clave.

- **Ojo: la suscripción de Claude (Pro o Max) no sirve como clave de API.** La API de Anthropic se contrata y se paga aparte, por uso, en la consola de Anthropic. Por foto sería un coste pequeño, pero no es gratis ni va incluido en la suscripción.
- **Cómo evitar el jardín de proveedores:** dos adaptadores detrás del «reconocedor de imágenes» (D18) cubren casi todo:
  1. **Anthropic**, con su API nativa.
  2. **Compatible con OpenAI**: el formato de API de OpenAI es el estándar de facto. Con configurar una URL base y una clave, el mismo adaptador sirve para OpenAI, OpenRouter y **modelos locales** (Ollama, LM Studio, llama.cpp server), que exponen ese mismo formato.
- Se decidirá cuando se llegue a la Fase 5, después de la prueba de concepto con OCR local.

## D21 · Proveedores de metadatos por tipo de catálogo — `propuesta`

Cada catálogo tiene sus fuentes detrás de la interfaz de «proveedor de metadatos» (D18). Candidatas:

| Catálogo | Fuentes | Notas |
|---|---|---|
| Libros | Google Books, Open Library | Google Books suele cubrir mejor el español; Open Library es abierta. Comprobar cuotas y condiciones. |
| Juegos de mesa | BoardGameGeek (API XML) | Comprobar condiciones de uso y si requiere registro. |
| Películas y series | **TMDB** | IMDb **no tiene una API pública gratuita**: su API oficial es de pago y para empresas. TMDB es la alternativa habitual en apps open source: es gratuita con registro, pide atribución y conviene comprobar su uso comercial. |
| Videojuegos | IGDB, RAWG… | A investigar en la Fase 6. |
| Cómics | Comic Vine, Grand Comics Database… | A investigar en la Fase 6. |

**Claves de API en una app open source:** nunca van en el repositorio (D03). Si un proveedor exige clave, cada instalación usa la suya, o en modo conectado la pone el servidor. Los proveedores sin clave (Open Library) funcionan siempre.

## D22 · Web pública y sitio de documentación — `abierta (lejano)`

Para el lanzamiento: una landing y un sitio de documentación (guía de usuario, instalación, autoalojamiento, referencia técnica).
- **Landing:** en Symfony, junto al servidor, o como página estática.
- **Documentación:** candidatos Docusaurus (React, que ya usaremos en la app), Starlight (Astro), VitePress o MkDocs Material.
- **Lo que importa ya:** toda la documentación se escribe en **Markdown dentro del repositorio**. Así cualquier generador la podrá publicar sin reescribirla. Los documentos de trabajo de `docs/` se irán convirtiendo en guías siguiendo Diátaxis (ver [README.md](README.md)).

## D23 · Entorno de desarrollo en Linux: Distrobox — `aceptada`

**Problema detectado el 24/09/2026.** El equipo de desarrollo usa **Ubuntu 20.04**. Tauri 2 necesita `libwebkit2gtk-4.1`, que no está disponible para Ubuntu 20.04 (llega en la 22.04). Además, Ubuntu 20.04 terminó su soporte estándar en 2025. Por otra parte:
- Rust, `gh` y el SDK de Android no están instalados.
- El JDK instalado es el 11; para compilar para Android hace falta el 17 o superior (Android Studio trae uno propio).

**Opciones.**

| Opción | A favor | En contra |
|---|---|---|
| **A. Actualizar a Ubuntu 24.04 LTS** | Lo más limpio: todo nativo, sin capas intermedias; sistema con soporte de nuevo. | Actualizar el sistema operativo lleva su tiempo y su riesgo (copia de seguridad antes). |
| B. Distrobox con Ubuntu 24.04 | No toca el sistema: un contenedor con integración gráfica, en el que las ventanas de Tauri se ven como apps normales. | Una capa más que entender; el SDK de Android y el USB del móvil dentro del contenedor dan más guerra. |
| C. Desarrollar en Windows y compilar Linux solo en la CI | No hay que tocar el Linux de casa. | Desarrollas en casa con Linux; no encaja. |

**Recomendación:** A. B es un buen plan provisional si prefieres no actualizar todavía.

**Decisión: B, Distrobox.** El portátil es un Slimbook con una distribución propia; actualizarlo exige reinstalar de cero, y no compensa el riesgo.
- Docker funciona sin sudo, así que Distrobox se instala en `~/.local` sin tocar el sistema.
- Se crea un contenedor Ubuntu 24.04 con todo lo necesario (dependencias de Tauri, Rust, Node, SDK y NDK de Android, JDK 17). Distrobox comparte con el sistema la carpeta personal, la red y el entorno gráfico, así que el código se edita desde el editor de siempre y las ventanas de Tauri aparecen como apps normales.
- **Android:** el móvil se conecta por **depuración inalámbrica** (Android 11 o superior), sin USB. Como el contenedor comparte la red del sistema, `adb` lo ve igual que desde fuera. Si hiciera falta el USB, se puede ejecutar el servidor de `adb` en el sistema (las *platform-tools* se descomprimen en la carpeta personal, sin sudo) y el contenedor se conecta a él.
- Los pasos exactos se documentarán en `docs/development/setup.md` a medida que se validen.

**Aparte, fuera del proyecto:** Ubuntu 20.04 ya no recibe actualizaciones de seguridad estándar. Ubuntu Pro es gratuito para uso personal (hasta 5 equipos) y amplía esas actualizaciones (ESM) sin reinstalar nada.

## D24 · Tres niveles: obra, edición y ejemplar — `aceptada`

**Contexto.** VISION.md separa la edición (lo que tiene ISBN) del ejemplar (la copia que alguien posee). Faltaba decidir si añadir por encima la **obra**: «El nombre del viento» en tapa dura en español y en ebook en inglés son dos ediciones de la misma obra.

**Decisión.** Tres niveles:
- **Obra** (`works`): el libro en abstracto. Tiene título y autores, y en el futuro, saga y número dentro de la saga.
- **Edición** (`editions` + `book_details`): una publicación concreta, con editorial, año, idioma, ISBN y portada. Es el `items` de D08.
- **Ejemplar** (`copies`): lo que hay en casa, con propietario, formato, unidades, ubicación y fecha de compra.

**Consecuencias.**
- «¿Lo tengo ya?» puede responder «sí, en otra edición».
- El estado de lectura (Fase 4) se marca sobre la obra: leer un libro no depende de la edición.
- Las sagas (Fase 4) se ordenan por obras.
- **Para no complicar el alta**, la app crea la obra automáticamente al dar de alta una edición nueva. Más adelante se podrán fusionar dos obras que resulten ser la misma.
- El esquema concreto está en [architecture/data-model.md](architecture/data-model.md).

## D25 · Operaciones: cambios de entidad con etiqueta de intención — `aceptada`

**Contexto.** En el hito 1.2 toda escritura pasa a ser una operación (D05). La forma que tenga condiciona la sincronización (D13), el servidor en PHP (D06) y los permisos. Lo que no depende de esta decisión: cada operación lleva un UUID v7, el identificador del dispositivo y una marca HLC, y se guarda en una tabla `operations` en la misma transacción que aplica sus cambios.

**Opciones.**

| | A. Cambios de entidad con etiqueta de intención | B. Operaciones con nombre |
|---|---|---|
| Forma | Un lote atómico de cambios `{entidad, id, crear/modificar/borrar, campos}`, con una etiqueta como `book.add` que solo sirve para el historial y para mostrarlo | Un tipo por acción (`book.add`, `copy.lend`, `copy.return`…), cada uno con sus datos y su lógica |
| Fusión de conflictos | Uniforme: el último en escribir gana, campo a campo (HLC). Se escribe una sola vez | Cada tipo de operación define cómo se fusiona |
| Permisos en el servidor | Por entidad y acción («un lector no puede borrar ejemplares»). Se comprueban sobre los cambios, así que no se pueden colar cambios bajo una etiqueta inocente | Por tipo de operación; el servidor vuelve a ejecutar la lógica de cada una |
| Código duplicado Rust/PHP | Poco: aplicar cambios genéricos y validar cada entidad | Mucho: cada operación, dos veces |
| Intención del usuario | Queda en la etiqueta | Explícita en el tipo |
| Referentes | CRDT por campos, cr-sqlite | Replicache, Linear |

**Decisión: A** (24/09/2026). Con dos implementaciones (Rust y PHP), B multiplica el código que hay que mantener en sintonía. A da gratis la fusión campo a campo que ya pedía D13. Las reglas de negocio que cruzan entidades (por ejemplo, «solo se presta un ejemplar disponible») se comprueban sobre el estado resultante, en ambos lados.

El formato concreto está en [architecture/operations.md](architecture/operations.md).

## D26 · Búsqueda por ISBN: Open Library + Google Books con clave opcional — `propuesta`

**Contexto (comprobado el 25/09/2026).**
- **Google Books sin clave** devuelve 429 («cuota diaria agotada»): las peticiones anónimas comparten una cuota global y no son fiables.
- **Open Library** funciona sin clave, pero su endpoint clásico `/api/books` ya no existe. Hay que usar `/isbn/{isbn}.json` (la edición) y `/authors/{id}.json` (el nombre de cada autor). Su cobertura de ediciones españolas es menor: por ejemplo, no tiene *El nombre del viento* de Plaza & Janés (9788401352188).

**Propuesta.**
- Se consultan **los dos proveedores a la vez** y se combinan campo a campo: primero Google Books (mejor en español), después Open Library. Portada: primero la grande de Open Library, que tiene más resolución que la miniatura de Google.
- **Google Books admite una clave de API opcional** (gratuita en Google Cloud) que se configura en los ajustes de la app. Se guarda solo en este dispositivo (`local_state`): no se sincroniza, no se exporta y nunca entra en el repositorio (D03, D18). Sin clave, se intenta igualmente, y si hay 429 se avisa.
- Cada proveedor que falle (red, cuota, formato) se informa como **aviso**, sin romper la búsqueda: si el otro responde, sus datos se usan igual.
- Los datos encontrados **rellenan el formulario**; nunca se guardan sin que el usuario pulse «Guardar».
- Se guardan los identificadores de cada fuente (`google_books_id`, `open_library_id`) para enlazar a sus fichas.
- Los datos de prueba de Open Library son respuestas reales. Los de Google Books siguen el formato documentado, porque no hubo cuota para grabar uno real: **hay que verificarlos con una clave**.

## D27 · Portadas: ficheros locales con nombre por hash — `propuesta`

- La portada se descarga al guardar, se guarda en `<datos de la app>/covers/` con nombre `<sha256>.<ext>`, y la edición registra solo ese nombre (campo `cover`).
- **Por qué el hash:** dos dispositivos que descargan la misma portada generan el mismo nombre, y al sincronizar (Fase 2) basta con pedir los ficheros que falten por nombre. Además, las operaciones no cargan con imágenes.
- **Seguridad:** solo se descargan URL `https` de los servidores de portadas conocidos (Open Library y Google Books), de menos de 5 MB y con tipo de imagen. Así nadie puede usar la app para hacer peticiones a otros sitios, ni a la red local.
- La interfaz muestra las portadas mediante el protocolo `asset` de Tauri, limitado a la carpeta `covers`.
- Pendiente: subir una portada propia desde un fichero (con el diálogo de ficheros del hito 1.6).

## D28 · Alta rápida: guarda lo encontrado, no duplica lo que ya existe — `propuesta`

El modo ráfaga (hito 1.4) prioriza la velocidad sin sacrificar el control:
- Se fijan una vez por sesión el propietario, el formato y la ubicación. Cada ISBN guarda esos valores **en el momento de escanearlo**: si se cambian a mitad de sesión, lo que ya estaba en cola no se altera.
- **Encontrado online:** se guarda al momento, sin confirmación, con los datos y la portada encontrados. Es la diferencia con el alta normal, donde el usuario revisa antes de guardar. Se corrige después desde la ficha.
- **Ya en el catálogo:** **no** se añade un ejemplar por defecto. Escanear algo que ya tienes suele ser un error o una comprobación («¿lo tengo?»), no una compra repetida. Hay un botón para añadirlo igualmente.
- **No encontrado:** queda pendiente, con un campo para escribir el título y guardarlo ahí mismo.
- Los ISBN se procesan **en cola y en orden**, uno a uno: se puede escanear más rápido de lo que responden los servicios.
- Los lectores de códigos USB funcionan igual: escriben el ISBN y pulsan Intro (D19).

## D29 · Editoriales como catálogo y búsqueda con FTS5 — `propuesta`

**Editoriales.** Pasan de ser texto libre en la edición a una tabla `publishers`, igual que los autores (`people`): se escriben una vez, se autocompletan y, cuando haya edición de catálogos, se podrán renombrar en un solo sitio. La migración convierte los textos existentes en editoriales, unificando las que solo se diferencian en mayúsculas.
- **Consecuencia:** las operaciones anteriores a este cambio guardan `publisher` como texto, un campo que el esquema actual ya no admite. Hoy no importa, porque no hay sincronización ni se vuelven a aplicar operaciones antiguas. Antes de la Fase 2 hay que decidir cómo versionar el formato de las operaciones (anotado en [architecture/operations.md](architecture/operations.md)).

**Búsqueda.** Índice de texto completo FTS5 de SQLite (`search_index`), con el tokenizador `unicode61` sin diacríticos: «senor» encuentra «Señor» y «garcia» encuentra «García». Cada libro indexa su título, subtítulo, título de la obra, autores, editorial e ISBN.
- El índice se actualiza **dentro de `ops::record`**, con cada operación, calculando qué ediciones toca (por ejemplo, renombrar un autor reindexa todos sus libros). Así también se mantendrá al día con las operaciones que lleguen por sincronización.
- Cada palabra buscada funciona como prefijo («tolk» encuentra «Tolkien»), y deben coincidir todas. Un ISBN, con o sin guiones, en formato de 10 o de 13 dígitos, se busca por su forma normalizada.
- Si cambia el formato del índice, se reconstruye solo al arrancar (versión guardada en `local_state`).

**Filtros:** formato, propietario y ubicación de los ejemplares, combinables con la búsqueda.

## D30 · Copias de seguridad (ZIP) y CSV — `propuesta`

**Copia de seguridad: un `.zip` con:**
- `backup.json`: todas las filas del catálogo, con los mismos nombres de campo que las operaciones; el registro de operaciones como historial; el formato (`library-backup`, versión 1) y la versión del esquema de la base de datos.
- `covers/`: las portadas que usa el catálogo.

**Restaurar:**
- Solo sobre un **catálogo vacío**. Mezclar dos catálogos es trabajo de la sincronización (Fase 2), no de una copia de seguridad.
- Solo si la copia se hizo con la **misma versión del esquema**. Más adelante se podrá migrar una copia antigua.
- Se restaura como **una única operación `backup.restore`** que crea todas las filas. Así pasa por la misma validación, el mismo índice de búsqueda y el mismo registro que cualquier otra escritura.
- El **historial** original viaja en el fichero, pero **no se restaura**: el catálogo restaurado empieza con la operación de restauración. Recuperarlo encaja con la sincronización y el versionado de operaciones (Fase 2).
- **Seguridad:** las rutas de dentro del ZIP nunca se usan para escribir en disco. Solo se aceptan portadas con nombre `<sha256>.<ext>`, y el nombre se recalcula a partir del contenido, lo que evita el ataque *zip slip* y los ficheros manipulados.

**CSV:**
- Una fila por ejemplar (una edición sin ejemplares ocupa una fila con las columnas del ejemplar vacías). Varios autores en una celda, separados por `;`. Cabeceras en inglés, estables, para que otras herramientas lo puedan leer: `title, subtitle, authors, publisher, published_year, language, isbn13, format, owner, quantity, location, acquired_on`.
- UTF-8 con BOM, para que las hojas de cálculo reconozcan tildes y eñes.
- **Importar añade**: cada fila válida es un alta, y si el ISBN ya existe, añade un ejemplar. Las filas con errores se informan con su número de línea y no detienen el resto. Sin formato, se usa `other`. Las portadas no viajan en el CSV.

**Limitación en Android (para la Fase 2):** allí el diálogo de ficheros devuelve rutas `content://` del sistema de almacenamiento, no rutas de disco, y el código actual lee y escribe con rutas normales. Exportar e importar en el móvil habrá que adaptarlo cuando llegue la app de Android.

## D31 · Operaciones replicables, versión de esquema y operación inicial — `propuesta`

Cada operación guarda la versión del esquema de la base de datos. Solo se sincronizan dispositivos con la misma versión. Las operaciones anteriores a la sincronización quedan como historial no replicable, y cada dispositivo registra una vez una operación `sync.baseline` con todo lo que ya tiene. Así un dispositivo nuevo se pone al día solo con operaciones, sin una copia completa aparte. Detalle en [architecture/sync.md](architecture/sync.md).

## D32 · Aplicar operaciones recibidas: campo a campo y el borrado gana — `propuesta`

Cada campo guarda la marca HLC que lo escribió por última vez, y una operación recibida solo escribe un campo si su marca es posterior. Si un dispositivo borra y otro edita a la vez, **gana el borrado** (lápidas). Lo que no se puede aplicar (por ejemplo, un ejemplar de un libro ya borrado) se salta sin romper la sincronización. Alternativa descartada: que la edición «resucite» lo borrado. Es más difícil de explicar y de implementar bien. Detalle en [architecture/sync.md](architecture/sync.md).

## D33 · Duplicados entre dispositivos: se conservan y se fusionan a mano — `propuesta`

Lo creado a la vez en dos dispositivos (un mismo autor, un mismo ISBN) queda duplicado tras sincronizar. Los nombres y el ISBN dejan de ser únicos en la base de datos, aunque la app sigue reutilizando lo existente al teclear. Habrá una opción para fusionar duplicados. Alternativas descartadas: rechazar uno (pierde datos) y fusionar automáticamente (puede juntar cosas distintas).

## D34 · Transporte local: HTTPS con certificado fijado por el QR — `propuesta`

Concreta D13. El escritorio sirve por HTTPS con un certificado propio, y la huella del certificado viaja en el QR (o en un código para pegar). El otro dispositivo solo acepta ese certificado. El emparejamiento intercambia secretos permanentes para las siguientes sincronizaciones. Implementado en el hito 2.1; protocolo completo en [architecture/sync.md](architecture/sync.md#transporte-d34-hito-21).

## D35 · Escáner de códigos con el plugin oficial de Tauri (solo móvil) — `propuesta`

- Se usa `tauri-plugin-barcode-scanner`, el oficial, que abre la cámara y lee EAN-13 (el código de barras de los libros es su ISBN). Pide el permiso de cámara la primera vez.
- Solo existe en Android e iOS, así que se añade únicamente al compilar para móvil. La interfaz pregunta a la app si está en un móvil y solo entonces muestra el botón de escanear. En el escritorio se sigue tecleando el ISBN o usando un lector USB (D19).
- Alternativa descartada: leer la cámara desde la web (`getUserMedia` + una librería de JavaScript). Depende de permisos del WebView menos fiables en Android, y el plugin nativo es más rápido y preciso.

## D36 · Lista de deseos básica, sincronizada — `propuesta`

- Entidad `wish`: título, ISBN, autor, nota y portada (URL). Se sincroniza como el resto (operaciones), así que lo que apuntas en la tienda aparece en casa.
- «¿Lo tengo ya?» consulta **solo la base de datos local**: funciona sin cobertura. Si no lo tienes, ofrece añadirlo al catálogo (con búsqueda online si hay red) o a la lista de deseos.
- Queda para la Fase 4: listas por persona, reservar regalos sin que lo vea el destinatario y pasar un deseo al catálogo conservando su historia.
