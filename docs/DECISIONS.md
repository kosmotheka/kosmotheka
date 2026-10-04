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
| D09 | Autenticación simple | aceptada (el alcance multibiblioteca lo amplía D38; Google, D77) |
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
| D21 | Proveedores de metadatos por tipo de catálogo | reemplazada por D62 |
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
| D36 | Lista de deseos básica, sincronizada | reemplazada por D61 |
| D37 | Reencontrar dispositivos buscando en la subred, no por mDNS | propuesta |
| D38 | Un servidor aloja varias bibliotecas desde el principio | aceptada |
| D39 | Biblioteca en la nube: sincroniza con la nube y por la red local | aceptada |
| D40 | La app conoce y aplica los permisos; el servidor los vuelve a comprobar | reemplazada por D55 |
| D41 | Filas del catálogo en el servidor: clave `(library_id, id)` | aceptada |
| D42 | Registro abierto en el servidor | aceptada |
| D43 | Invitaciones de un solo uso que caducan en 24 horas | aceptada |
| D44 | El lector escribe solo lo suyo: deseos y lecturas | reemplazada por D55 |
| D45 | El editor solo borra lo que creó | aceptada |
| D46 | Varias bibliotecas en la app, como entidad en una sola base de datos | aceptada |
| D47 | Cambio rechazado por el servidor: se corrige solo en ese dispositivo | reemplazada por D73 |
| D48 | Producción con nginx + PHP-FPM + PostgreSQL, como en desarrollo | aceptada |
| D49 | Verificación obligatoria del email con un código en la app | aceptada |
| D50 | Recuperar la contraseña con un código en la app; revoca los demás dispositivos | aceptada |
| D51 | Superadministrador del servidor, con acceso a todo | aceptada |
| D52 | Panel de administración web con EasyAdmin y KPI | aceptada |
| D53 | ¿Una app web además de las nativas? | abierta |
| D54 | La biblioteca: la unidad que se comparte, se sincroniza y tiene permisos | aceptada |
| D55 | Personas, roles y peticiones, también sin servidor | aceptada |
| D56 | Cada dispositivo firma sus operaciones | aceptada |
| D57 | Dos formas de usar la app; la nube es un dispositivo más | aceptada |
| D58 | Copias sin claves y recuperación del control | aceptada |
| D59 | Organizaciones: la puerta abierta, sin construirlas | aceptada |
| D60 | Tipos de catálogo: lista cerrada y «Otro»; el alta empieza por buscar | aceptada |
| D61 | Series y estados: lo tengo, me falta, lo deseo | aceptada |
| D62 | Proveedores: todos los posibles, con las claves en el servidor | aceptada |
| D63 | Distribución: Google Play, sin iOS | aceptada |
| D64 | La documentación de trabajo, fuera de la versión pública | abierta |
| D65 | Borrar una biblioteca y salir de ella | aceptada |
| D66 | Registro de actividad para poder responder de lo que pasó | aceptada |
| D67 | Enlaces para compartir y etiquetas QR: son de la biblioteca | aceptada |
| D68 | Marcas por persona: estado, valoración y favorito | aceptada |
| D69 | Reserva de regalos oculta solo en pantalla | aceptada |
| D70 | Nombre del proyecto: Kosmotheka | aceptada |
| D71 | Dejar de compartir quita el acceso y borra la biblioteca en el otro dispositivo | aceptada |
| D72 | La marca (nombre y logo), fuera de la AGPL y protegida | propuesta |
| D73 | Cómo se comprueban los permisos, sin servidor y con él | aceptada |
| D74 | Publicar versiones: escritorio en GitHub Releases, Android a Google Play, ambos automáticos | propuesta |
| D75 | Avisos al momento: Mercure con la app abierta, notificaciones push en Android | aceptada |
| D76 | La cuenta en la biblioteca: enlace automático y la nube vincula los dispositivos nuevos | aceptada |
| D77 | Entrar con email o con Google | aceptada |
| D78 | Desbloqueo con biometría y segundo factor, opcionales desde el principio | aceptada (revisada el 02/10/2026: `otphp` en lugar de `scheb/2fa-bundle`) |
| D79 | Dominios: `.app` es lo que usa la app, `.com` es lo que se lee | aceptada |
| D80 | Correo: `info@kosmotheka.com` reenviado a Gmail, envíos con Brevo | aceptada |
| D81 | Dar un dispositivo a otra persona: la identidad es de la instalación y se puede empezar de cero | aceptada |
| D82 | Portadas propias (foto o fichero) y cómo viajan las imágenes entre dispositivos | abierta |
| D83 | Google Books sin clave no basta: clave propia, servidor solo con cuenta y la BNE | aceptada (la BNE, por investigar) |
| D84 | Acceso a la nube: id propio de la nube y tabla de acceso; el servidor no se fía de ningún id del cliente | aceptada |
| D85 | Adiós al id fijo de «Mi biblioteca»: toda biblioteca nace con id aleatorio | aceptada |
| D86 | Copias de seguridad cifradas con contraseña, activado por defecto | aceptada |
| D87 | Cada operación encadena la anterior de su autor | aceptada |
| D88 | Emparejar una vez: los dispositivos de una biblioteca se reconocen solos en la red local | abierta |

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

> Matizada el 25/09/2026 por D55 y D57: los permisos también se garantizan sin servidor, entre los propios dispositivos; la autoridad del servidor queda para las bibliotecas en la nube.

- La app **siempre** lee y escribe en su base de datos local (SQLite). La interfaz nunca espera a la red.
- **Modo local** (sin cuenta): la app funciona sola. Es un producto completo para quien quiere su catálogo privado.
- **Modo conectado** (con cuenta en un servidor): la app sigue escribiendo en local, pero además envía cada cambio al servidor como una **operación** («añadir ejemplar», «borrar edición»…). El servidor **vuelve a aplicar la operación con sus propias reglas**: si el usuario no tiene permiso, la rechaza, y el cliente deshace el cambio local. El servidor es la fuente de la verdad; el cliente es una réplica que puede adelantarse.

Es el patrón de mutaciones optimistas que el servidor vuelve a aplicar con autoridad (Linear, Replicache). Resuelve el problema de permisos que hizo descartar el local-first en la ideación.

**Motor de sincronización propio**, no una librería externa: el objetivo es aprender los conceptos. Antes de diseñarlo hemos revisado los motores existentes y las ideas que tomamos de ellos (ver [research/sync-engines.md](research/sync-engines.md)).

**Consecuencias.**
- El modo sin conexión forma parte de la arquitectura desde el primer día; no es una caché que se añade al final.
- El modelo de datos funciona igual en SQLite y en PostgreSQL (D08).
- Los identificadores se generan en el cliente (UUID v7), no son autoincrementales. *Excepción:* las marcas por persona tienen un id derivado de persona y obra (D68).
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

> Ampliada el 01/10/2026 por D77: además de email y contraseña, entrar con Google.

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

## D21 · Proveedores de metadatos por tipo de catálogo — `reemplazada por D62`

> Reemplazada el 25/09/2026 por D62, tras investigar las condiciones de cada proveedor.

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

**Ampliación con varias bibliotecas (Fase 4, hitos 4.3 y 4.6), propuesta:**
- **Formato versión 3:** la copia guarda una o varias bibliotecas, cada una con su id, su nombre, sus filas y su registro. La copia de una biblioteca y la **copia de todo el dispositivo** son el mismo formato: la segunda lleva todas.
- **No lleva** los dispositivos emparejados, las retiradas de acceso (D71) ni la identidad del dispositivo: la copia es del catálogo (D58).
- **Restaurar**, biblioteca a biblioteca:
  - vuelve **con su mismo id**, para que al emparejar otra vez se una con las demás copias en lugar de duplicarse;
  - si aquí se había **borrado**, vuelve con un id nuevo, porque el borrado sigue valiendo para el id antiguo;
  - si aquí **ya existe con libros**, se salta y se dice: juntar dos catálogos es cosa de la sincronización;
  - si su **nombre** ya lo usa otra biblioteca, se le añade un número: «Club (2)».
- Las copias de **versión 2** (una biblioteca sin id ni nombre) se siguen restaurando, en la biblioteca activa si está vacía.
- **Restaurar no trae los roles** (añadido el 27/09/2026 con D73 y D58): los miembros vuelven sin `role` ni `can_share`, porque apuntarían a dispositivos que no están, y quien restaura se quedaría como lector de su propia biblioteca. Quien restaura puede activar los roles de nuevo. Si la copia se une a una biblioteca compartida que ya tenía roles, esos roles no cambian.
- **Versiones del esquema:** se aceptan copias hechas desde la migración 9, porque las posteriores no cambian las tablas del catálogo. Si una migración futura las cambia, sube ese mínimo o convierte la copia.

**Limitación en Android (para la Fase 2):** allí el diálogo de ficheros devuelve rutas `content://` del sistema de almacenamiento, no rutas de disco, y el código actual lee y escribe con rutas normales. Exportar e importar en el móvil habrá que adaptarlo cuando llegue la app de Android.

## D31 · Operaciones replicables, versión de esquema y operación inicial — `propuesta`

Cada operación guarda la versión del esquema de la base de datos. Solo se sincronizan dispositivos con la misma versión. Las operaciones anteriores a la sincronización quedan como historial no replicable, y cada dispositivo registra una vez una operación `sync.baseline` con todo lo que ya tiene. Así un dispositivo nuevo se pone al día solo con operaciones, sin una copia completa aparte. Detalle en [architecture/sync.md](architecture/sync.md).

**Precisado el 03/10/2026 (DAT-003, auditoría 2026-10-03-3):** la versión que se compara para sincronizar es la de los dos extremos (la petición de `pull`, el emparejamiento), no la que guarda cada operación. La de cada operación solo dice en qué versión se escribió. La firma no la cubre, así que no decide nada por sí sola: lo que decide es que la operación valide con el esquema actual. Una escrita en una versión anterior o igual se acepta si valida; una que dice venir de una versión posterior se rechaza. Antes se exigía la misma versión también a cada operación, y tras cualquier actualización de la app el historial anterior dejaba de viajar: un dispositivo emparejado después no recibía la biblioteca.

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

## D36 · Lista de deseos básica, sincronizada — `reemplazada por D61`

> Reemplazada el 25/09/2026 por D61: el deseo pasa a ser una marca por persona sobre algo que falta.

- Entidad `wish`: título, ISBN, autor, nota y portada (URL). Se sincroniza como el resto (operaciones), así que lo que apuntas en la tienda aparece en casa.
- «¿Lo tengo ya?» consulta **solo la base de datos local**: funciona sin cobertura. Si no lo tienes, ofrece añadirlo al catálogo (con búsqueda online si hay red) o a la lista de deseos.
- Queda para la Fase 4: listas por persona, reservar regalos sin que lo vea el destinatario y pasar un deseo al catálogo conservando su historia.

## D37 · Reencontrar dispositivos buscando en la subred, no por mDNS — `propuesta`

**Contexto.** Si el escritorio cambia de IP (el router se la reasigna), el móvil deja de encontrarlo en la dirección guardada. El plan (D13) era descubrirlo por mDNS. Pero en Android, recibir multicast exige un «multicast lock» del sistema, que solo se obtiene con código nativo (Kotlin): sin él, muchos móviles descartan las respuestas mDNS.

**Propuesta.** Cuando ninguna dirección guardada responde, el dispositivo **recorre su propia subred** (como mucho 1024 direcciones, 64 a la vez, con tiempos de espera cortos) llamando al puerto conocido del otro. Solo acepta al que presenta **el certificado fijado** y **el identificador de dispositivo** guardados. Si lo encuentra, guarda la nueva dirección y sincroniza.
- Ventajas: funciona en cualquier sistema sin código nativo ni permisos extra, se puede probar automáticamente, y la seguridad no cambia (la huella del certificado sigue siendo la que manda).
- Inconvenientes: tarda unos segundos, y solo encuentra al otro si está en la misma subred y en el mismo puerto (el puerto se guarda y se mantiene entre ejecuciones).
- mDNS queda como mejora posible si algún día hace falta; exigiría un pequeño plugin nativo para Android.

**Además en el 2.3:** emparejar escaneando el QR con la cámara del móvil, y sincronizar al volver a la app (cuando pasa a primer plano), no solo al abrirla.

## D38 · Un servidor aloja varias bibliotecas desde el principio — `aceptada`

En el servidor, todo dato pertenece a una **biblioteca**, y cada usuario es miembro de una o varias con un rol (administrador, editor, lector). Cuesta poco ahora (una columna y un filtro en cada consulta) y deja preparados el servicio público, las ludotecas y las asociaciones sin migraciones dolorosas. Matiza D09: la autenticación sigue siendo simple (usuario y contraseña, un token por dispositivo), pero el modelo de datos ya no es de una sola familia.

## D39 · Biblioteca en la nube: sincroniza con la nube y por la red local — `aceptada`

> **Cerrada el 01/10/2026**, al preparar la Fase 7 ([plans/fase-7.md](plans/fase-7.md)). Sustituye a la versión de abajo («sincroniza solo con él»), que se conserva para saber de dónde viene.

**Decisión:** una biblioteca que está en la nube **sigue sincronizándose también por la red local**, con los dispositivos con los que se comparte. La app tiene que funcionar sin cobertura y entre dispositivos, como siempre. Es lo que sale de no cambiar la forma de sincronizar (decisión del maintainer del mismo día): la nube es un sitio más que tiene la biblioteca (D57), y cada sitio juzga lo que recibe con las mismas reglas (D73).

**Un cambio que llega por varios caminos no se aplica dos veces:**
- cada operación tiene su id y la firma de su autor (D56): llegue por la nube, por otro dispositivo o por los dos, es la misma operación y nadie la puede alterar por el camino;
- el vector de versiones (por biblioteca y por autor) evita reenviar lo que el otro ya tiene: si el móvil A sube un cambio a la nube y el móvil B lo baja, cuando B sincronice con la nube o con A no les manda nada;
- si aun así llega dos veces (dos dispositivos que suben a la vez), la fusión ignora una operación ya conocida, y el resultado es el mismo gane quien gane la carrera.

**Consecuencias:** sin trabajo nuevo en el protocolo; es lo que ya pasa con tres dispositivos por la red local. Si el VPS se cae, en casa se sigue sincronizando. La retirada de acceso (D71) llega por cualquiera de los dos caminos.

**Versión anterior (25/09/2026, reabierta):**

> **Reabierta** el 25/09/2026 en la [revisión del alcance](historico/revision-alcance.md): su motivo era que sin servidor no había permisos, y con D55 los dispositivos también los comprueban. Falta decidir si una biblioteca en la nube puede seguir sincronizándose por wifi.

Una biblioteca sin servidor se sincroniza por la wifi entre dispositivos, como en la Fase 2. En cuanto se conecta a un servidor, **el servidor pasa a ser la única vía de sincronización y el árbitro de los permisos**, y la sincronización por wifi de esa biblioteca se desactiva. Así ningún cambio no permitido puede pasar de un dispositivo a otro sin que el servidor lo compruebe. Alternativa descartada: mantener las dos vías (más flexible si el VPS se cae, pero los permisos solo se garantizarían a medias y la lógica se complica).

## D40 · La app conoce y aplica los permisos; el servidor los vuelve a comprobar — `reemplazada por D55`

> Reemplazada el 25/09/2026 por D55, que extiende la idea a todos los dispositivos, con servidor o sin él.

La app sabe qué rol tiene su usuario en cada biblioteca (le llega del servidor) y **no ofrece ni registra** lo que ese rol no puede hacer: un lector no ve «Borrar». El servidor comprueba de nuevo cada operación y rechaza las que no cumplan, por si una app estuviera manipulada.

**Por qué no «dejar hacer y deshacer después»** (lo que planteaba D05): con «el borrado gana» (D32), un borrado aplicado en local no se puede deshacer limpiamente, porque la lápida impediría recrear la fila. Si una app legítima nunca genera operaciones prohibidas, no hay nada que deshacer. Lo que un rol no puede hacer directamente se hace como **solicitud** (hito 3.4): por ejemplo, «pido borrar este libro», que es una operación permitida y que un administrador aprueba.

## D41 · Filas del catálogo en el servidor: clave `(library_id, id)` — `aceptada`

**Contexto:** en PostgreSQL todas las tablas del catálogo llevan `library_id` (D38). Los ids son UUID v7 generados en cada dispositivo y en la práctica no se repiten, pero el servidor recibe ids de clientes que no controla.

| Opción | A favor | En contra |
|---|---|---|
| Clave primaria solo `id` | Claves foráneas de una columna, como en SQLite | Si llega un id que ya existe en otra biblioteca, hay que detectarlo en el código; si se escapa, un cambio toca datos ajenos |
| **Clave primaria `(library_id, id)`** | La base de datos garantiza que una biblioteca nunca pisa filas de otra, aunque un cliente mande un id ajeno por error o a propósito | Las claves foráneas llevan dos columnas |

**Decisión:** `(library_id, id)` en todas las tablas del catálogo y en sus tablas de soporte (`field_versions`, `tombstones`); las claves foráneas entre tablas del catálogo incluyen `library_id`, así que una fila tampoco puede apuntar a otra biblioteca. En el cliente (SQLite) no cambia nada: cada base local es una sola biblioteca.

**Consecuencias:** el motor de aplicación del servidor (Fase 7) filtra y escribe siempre con `library_id`; no hay consulta del catálogo sin él.

## D42 · Registro abierto en el servidor — `aceptada`

**Contexto:** D38 fija usuario y contraseña, con un token por dispositivo. Faltaba decidir cómo se crea una cuenta.

| Opción | A favor | En contra |
|---|---|---|
| Solo por invitación (primer administrador por consola) | Nadie de fuera entra; poco que proteger | Cierra, por ahora, la puerta al servicio público |
| **Registro abierto** | Cualquiera con la URL crea su cuenta y su biblioteca: es la base del servicio público | Exige protección contra abusos desde el principio |
| Configurable por variable de entorno | Las dos cosas | Más trabajo ahora |

**Decisión:** registro abierto. Quien se registra crea su cuenta y puede crear sus bibliotecas, de las que es administrador; las invitaciones siguen siendo la forma de unirse a una biblioteca ajena.

**Consecuencias:** entra en la Fase 7 lo necesario para un registro público: límite de peticiones por IP en el alta y el inicio de sesión (`symfony/rate-limiter`, con `trusted_proxies` bien configurado, como en Riddle). Quedaban por decidir (resuelto en D49–D51): si se verifica el email (y con ello el envío de correo), cómo se recupera una contraseña olvidada y si el servidor tiene un administrador global además de los administradores de cada biblioteca.

## D43 · Invitaciones de un solo uso que caducan en 24 horas — `aceptada`

**Contexto:** para unirse a una biblioteca ajena hace falta una invitación (D42). Se entrega como QR con la URL del servidor, un token y el rol con el que se entra.

| Opción | A favor | En contra |
|---|---|---|
| 7 días, un solo uso | Margen para enviarla a alguien que no está delante | Si se filtra, sirve durante una semana |
| **24 horas, un solo uso** | Si se filtra (una captura, un chat), deja de servir pronto y solo para una persona | Si no se abre ese día, hay que generar otra |
| Sin caducidad, varios usos | Un solo QR para toda la familia | Quien lo consiga entra hasta que se revoque a mano |

**Decisión:** un solo uso y 24 horas. El administrador puede revocarla antes y generar otra en cualquier momento. En la base de datos solo se guarda el hash del token.

**Consecuencias:** la app debe explicar, al escanear una invitación caducada o ya usada, que se pida otra al administrador (entrada nueva en `docs/design/errors.md` cuando se implemente, en la Fase 7).

## D44 · El lector escribe solo lo suyo: deseos y lecturas — `reemplazada por D55`

> Reemplazada el 25/09/2026 por D55 (roles y peticiones) y D61 (deseos y lecturas por persona).

**Contexto:** el lector no modifica el catálogo (obras, ediciones, ejemplares…). Quedaba por decidir si puede apuntar cosas propias.

| Opción | A favor | En contra |
|---|---|---|
| **Sí, lo suyo** | Puede usar la lista de deseos y (Fase 4) llevar sus lecturas | Hay que saber quién creó cada deseo y cada lectura |
| Solo leer | Lo más simple | Sin deseos ni lecturas; tendría que pedírselo a un editor |
| Deseos ahora, lecturas más adelante | Aplaza parte de la decisión | Deja la regla a medias |

**Decisión:** el lector crea, edita y borra sus deseos y (Fase 4) sus lecturas; no toca el catálogo ni lo que apuntaron otros.

**Consecuencias:** el servidor guarda quién creó cada fila (el usuario de la operación que la creó) y lo usa para los permisos; no hace falta un campo nuevo en `spec/`, porque el autor de cada operación ya lo conoce el servidor por el token del dispositivo. La app (D40) ofrece al lector solo esas acciones.

## D45 · El editor solo borra lo que creó — `aceptada`

**Contexto:** el editor crea y edita todo el catálogo. Un borrado no tiene vuelta atrás, porque el borrado gana (D32).

| Opción | A favor | En contra |
|---|---|---|
| **Borra lo que creó él** | Corrige sus propios errores sin esperar a nadie | Para lo ajeno necesita una solicitud |
| Nunca borra | Lo más seguro | Hasta para deshacer su propio error espera a un administrador |
| Borra todo | Lo más cómodo | Cualquier editor borra libros ajenos sin vuelta atrás |

**Decisión:** el editor borra solo las filas que creó él; para borrar lo ajeno manda una solicitud que aprueba un administrador (D55). El administrador borra todo.

**Consecuencias:** usa el mismo registro de quién creó cada fila que D44. Un borrado que arrastra filas relacionadas (borrar un libro borra su edición y sus ejemplares) solo se permite si todas las filas afectadas son suyas; si no, se convierte en solicitud. La app (D40) muestra «Borrar» o «Pedir que se borre» según el caso.

## D46 · Varias bibliotecas en la app, como entidad en una sola base de datos — `aceptada`

> Ampliada el 25/09/2026 por D54: la biblioteca es además la unidad que se comparte, se sincroniza y tiene permisos, también sin cuenta.

**Qué se busca:** que en la app se puedan tener varias bibliotecas a la vez; por ejemplo, la de la familia y una con amigos. Un usuario puede estar en varias, con un rol distinto en cada una (D38).

**Decisión:**
- **Una sola base de datos** en el dispositivo, con una entidad **biblioteca**; nada de una base por biblioteca.
- Al unirse a una biblioteca del servidor teniendo libros propios, la local se conserva (por ejemplo, con «(local)» detrás del nombre) y se puede **mover un ítem a otra biblioteca**.
- **Cada biblioteca tiene lo suyo**: sus propias obras, ediciones y ejemplares, aunque el mismo libro esté en varias. Una corrección en una no cambia las demás, y cada fila pertenece a una sola biblioteca (encaja con D41). «¿Lo tengo?» y el buscador pueden mirar en todas las bibliotecas del usuario a la vez.
- **`library_id` en todas las tablas** locales, con clave `(library_id, id)`, igual que en el servidor (D41): cada consulta filtra por biblioteca sin joins y cliente y servidor se parecen.
- **Wifi: se elige por biblioteca.** El emparejamiento sigue siendo entre dispositivos, pero para cada dispositivo emparejado se elige qué bibliotecas locales se comparten con él. Al emparejar desde una biblioteca, esa queda marcada; las demás no se envían hasta marcarlas. Nada sale sin que el usuario lo diga.
- **Mover un libro crea ids nuevos** en el destino, y la app guarda de dónde vino. Se puede mover de ida y vuelta, porque las lápidas del origen (D32) no afectan a ids nuevos. El historial anterior se queda en la biblioteca de origen.
- ~~**Migración con continuidad** (cambiado el 26/09/2026; antes era «sin continuidad»): al actualizar, cada dispositivo pasa sus datos a una biblioteca con **el mismo id fijo en todos los dispositivos**. Los que ya se sincronizaban tienen las mismas filas con los mismos ids, así que al volver a emparejar se funden sin duplicados. Riesgo aceptado: dos instalaciones que nunca se sincronizaron juntarían sus catálogos si se emparejan y comparten esa biblioteca; hoy solo hay una en uso.~~ **Reemplazado el 03/10/2026 por D85:** toda biblioteca nace con id aleatorio.
- **Interfaz: una biblioteca a la vez, con selector.** Catálogo, deseos y altas son de la biblioteca activa; el buscador tiene la opción «en todas» y «¿Lo tengo?» mira siempre en todas.
- **Copias: las dos cosas.** Una copia completa del dispositivo, con todas las bibliotecas, y además exportar e importar una biblioteca suelta (copia o CSV), para compartirla o restaurarla sin tocar las demás. Una biblioteca suelta se puede restaurar si no existe ya en el dispositivo.
- **Miembro y usuario, enlace opcional.** Un miembro (dueño de ejemplares) puede enlazarse con un usuario de esa biblioteca, o no (un niño sin cuenta). Sirve para «mis libros» y, en la Fase 4, para asociar las lecturas a la persona. En una biblioteca local no hay enlace. Los permisos sobre «lo suyo» siguen basándose en quién creó la fila (D44, D45).

**Consecuencias:**
- Una biblioteca conectada a un servidor sincroniza solo con él (D39); las locales, por wifi con los dispositivos elegidos. Vectores de versión e historial son por biblioteca.
- Sacar un libro de una biblioteca del servidor es un borrado allí, así que sigue D45.
- Cambia el modelo de datos de la app (migración nueva) y el diseño (selector de biblioteca, «Mover a otra biblioteca»; ver `docs/design/brief.md`).

## D47 · Cambio rechazado por el servidor: se corrige solo en ese dispositivo — `reemplazada por D73`

> **Reemplazada el 01/10/2026 por D73**, al preparar la Fase 7: la nube juzga cada operación igual que cualquier dispositivo, así que lo no permitido pasa a petición en todos los sitios y el dispositivo del autor se corrige solo, al recalcular la biblioteca desde su registro. La nube nunca corrige a un dispositivo; solo rechaza lo inválido. Ver el añadido de D73.

> Ampliada el 25/09/2026 por D55: lo rechazado no se descarta, pasa a petición; el dispositivo del autor lo deshace hasta que se apruebe. Vale igual entre dispositivos que con el servidor.

**Contexto:** con D40 la app no genera cambios que su rol no permite, pero puede haberlos: por ejemplo, un editor sin conexión borra un libro suyo mientras un administrador le baja a lector. Al sincronizar, el servidor rechaza el borrado, pero en ese dispositivo el libro ya no está, y la lápida (D32) impediría que volviera por la vía normal.

| Opción | A favor | En contra |
|---|---|---|
| **Corrección del servidor** | Solo se deshace lo rechazado; nadie más ve nada | Un mecanismo nuevo en el cliente |
| Descargar de cero la biblioteca | Muy simple | Se pierden también los cambios válidos aún no subidos |
| Convertirlo en solicitud | Aprovecha las solicitudes | Lo más complejo; la corrección hace falta igualmente si se rechaza |

**Decisión:** el servidor no aplica la operación rechazada y responde a ese dispositivo con el motivo y con el **estado válido de las filas afectadas** (sus campos y versiones). La app lo aplica encima, deshaciendo su cambio; si era un borrado, **quita la lápida local y restaura la fila**. La operación rechazada queda en el historial local marcada como rechazada y no se vuelve a enviar. La app avisa con el motivo (por ejemplo, «No se pudo borrar X: ya no tienes permiso»).

**Consecuencias:**
- Es la única forma en que una lápida puede levantarse, y solo por orden del servidor, en una biblioteca conectada a él (D39). La sincronización por wifi nunca lo hace.
- Antes de programarlo se escriben casos de conformidad en `spec/` (rechazo de una creación, de una edición y de un borrado, y un rechazo seguido de cambios válidos).
- Entrada nueva en `docs/design/errors.md` cuando se implemente (Fase 7).

## D48 · Producción con nginx + PHP-FPM + PostgreSQL, como en desarrollo — `aceptada`

**Contexto:** hay que empaquetar el servidor para el VPS y para quien quiera alojar el suyo. En desarrollo ya se usa nginx + PHP-FPM + PostgreSQL, como en Riddle y rpgutilities.

| Opción | A favor | En contra |
|---|---|---|
| FrankenPHP (modelo de Koillection) | Una imagen con HTTPS automático; dos servicios para autoalojarlo | Distinto de desarrollo y de lo que el mantenedor ya opera |
| **nginx + PHP-FPM** | Igual en desarrollo y en producción; lo conocido | Tres contenedores; el HTTPS lo pone un proxy delante |

**Decisión:** nginx + PHP-FPM + PostgreSQL también en producción. El HTTPS lo termina un proxy delante (el que ya haya en el VPS, o Certbot).

**Consecuencias:** sustituye a la idea de «una imagen Docker con FrankenPHP» del ROADMAP. En la Fase 7 hacen falta imágenes de producción (sin Xdebug ni dependencias de desarrollo), un `docker-compose` de producción y una guía de instalación para quien se aloje el suyo, incluido cómo poner el proxy con HTTPS.

## D49 · Verificación obligatoria del email con un código en la app — `aceptada`

**Contexto:** con registro abierto (D42) cualquiera puede crearse una cuenta. El registro se hace desde la app; el servidor no tiene páginas web de cuenta.

| Opción | A favor | En contra |
|---|---|---|
| **Obligatoria** | Frena cuentas falsas; el email sirve después para recuperar la contraseña | Hay que configurar el envío de correo |
| Sin bloquear | Más cómodo | Cualquiera se registra con un email ajeno |
| No verificar | Lo más simple | Sin recuperación por email y más expuesto a abusos |

**Decisión:** hasta confirmar el email, la cuenta no puede usarse. Se confirma con un **código enviado por correo que se teclea en la app**, como los servicios que envían un código por SMS; así no hace falta una página de confirmación. Del patrón de rpgutilities se conserva el **`UserChecker`**, que no deja entrar a una cuenta sin confirmar (ni a una bloqueada, D51).

**Consecuencias:**
- El código caduca, admite pocos intentos y se puede pedir otro, con límite de peticiones (D42).
- **Contra bots**, sin honeypot (en una app no sirve: los bots atacan la API directamente): límite de peticiones por IP (D42); las cuentas sin confirmar se borran solas al cabo de 24 horas; y límite de correos enviados por dirección, para que nadie use el servidor para inundar el correo de otro. Si hay abusos, se añadirá Cloudflare Turnstile.
- **Cómo se envía el correo:** con Brevo (D80). `symfony/mailer` permite cambiar de proveedor sin tocar el código, con `MAILER_DSN` en `.env.local`.

## D50 · Recuperar la contraseña con un código en la app; revoca los demás dispositivos — `aceptada`

**Contexto:** la contraseña solo hace falta para dar de alta un dispositivo (D38); los que ya tienen token siguen funcionando.

**Decisión:** el mismo mecanismo que D49: se pide la recuperación desde la app, llega un código por correo, se teclea en la app junto con la contraseña nueva. Al cambiarla se revocan los tokens de todos los demás dispositivos de la cuenta, por si el motivo era que alguien la conocía.

**Consecuencias:** un dispositivo revocado pide iniciar sesión de nuevo. **No pierde nada**: sus cambios sin subir siguen en su base local y se envían en cuanto vuelve a entrar. La app lo tiene que explicar así (entrada nueva en `docs/design/errors.md`, Fase 7). El envío del correo depende de lo que se decida en D49.

## D51 · Superadministrador del servidor, con acceso a todo — `aceptada`

**Contexto:** con registro abierto (D42) habrá cuentas y bibliotecas de gente que el operador no conoce. Alguien tiene que poder bloquear una cuenta que abusa o ayudar a quien perdió el acceso.

**Decisión:** el patrón típico de Symfony, como en rpgutilities: `ROLE_SUPER_ADMIN` en la jerarquía de roles, por encima de `ROLE_ADMIN` y con `ROLE_ALLOWED_TO_SWITCH` para suplantar a un usuario al dar soporte. Puede gestionar cuentas y bibliotecas y ver y editar cualquier biblioteca. Se asigna por consola, nunca desde la API.

**Consecuencias:**
- `ROLE_SUPER_ADMIN` es un rol de Symfony sobre la cuenta; los roles de cada biblioteca (administrador, editor, lector) viven en la membresía (D38). Son dos niveles distintos.
- Si el servidor se ofrece como servicio público, su política de privacidad debe decir que el operador tiene este acceso.
- Dónde administra: D52.

## D52 · Panel de administración web con EasyAdmin y KPI — `aceptada`

**Decisión:** un panel web para el superadministrador (D51) con la gestión típica de EasyAdmin (cuentas, bibliotecas, dispositivos, invitaciones) y además KPI, como el panel de rpgutilities.

**KPI elegidos:**
- **Crecimiento:** cuentas registradas, confirmadas y sin confirmar; bibliotecas creadas; altas por semana o mes.
- **Actividad:** usuarios y dispositivos activos (7 y 30 días), operaciones recibidas por día, última sincronización por biblioteca.
- **Contenido:** libros, ejemplares y deseos en total y por biblioteca; bibliotecas más grandes; tamaño de la base de datos.
- **Salud y abuso:** operaciones rechazadas (D47) y su motivo, peticiones cortadas por el límite, correos enviados y fallidos, cuentas bloqueadas.

Los de salud y abuso necesitan que el servidor **registre esos eventos** (rechazos, límites, correos) en una tabla propia; los demás salen de las tablas que ya habrá.

**Cuándo:** en la Fase 11 (revisado el 25/09/2026; antes estaba previsto con el despliegue). Hasta entonces, comandos de consola en `src/Command/`. El registro de eventos para los KPI se va añadiendo desde la Fase 7, con cada funcionalidad, para que el panel tenga datos desde el primer día.

**Accesos del superadministrador:** cada entrada en una biblioteca de la que no es miembro queda registrada (quién, cuándo, qué biblioteca, si fue suplantando a alguien) y solo se ve en el panel. Sus cambios aparecen además en el historial con su nombre, como los de cualquiera.

## D53 · ¿Una app web además de las nativas? — `abierta`

> Tras la revisión del alcance (25/09/2026), la web solo aparece en el escenario del servicio para ludotecas o colegios (Fase 11). Sin urgencia.

**Planteamiento del mantenedor:** las aplicaciones suelen tener también una versión web, aunque existan apps para Windows, Linux y Android. ¿Se podría crear la web con el mismo código que la app de Tauri? El objetivo personal del proyecto es aprender a crear apps nativas, pero no hay que descuidar el futuro producto de código abierto con servidor.

**Por debatir, al final y aparte**, cuando el resto del plan esté discutido. Afecta a D46 (varias bibliotecas), al servidor (una web necesita el servidor, porque no tiene base de datos local) y a la administración (D52).

## D54 · La biblioteca: la unidad que se comparte, se sincroniza y tiene permisos — `aceptada`

**Contexto:** se dio por hecho que cada persona tendría varias bibliotecas (una personal de rol, una familiar, la de videojuegos del hijo) y no se reflejó en el diseño: la app trataba el dispositivo entero como una sola biblioteca. Detalle del debate en la [revisión del alcance](historico/revision-alcance.md).

**Decisión:**
- La biblioteca es **lo que se comparte**: qué se sincroniza, con quién y con qué permisos, con cuenta y sin ella. Amplía D46 (una sola base, `library_id` en todas las tablas, selector, mover con ids nuevos, copias completas y por biblioteca).
- **Vive completa en cada sitio que la tiene.** Compartir es añadir un sitio más: emparejar un dispositivo, invitar a alguien o subirla a la nube son lo mismo.
- **Cada operación pertenece a una biblioteca**, y el vector de versiones es por biblioteca.
- **Tiene un propietario**, que hoy siempre es una persona (D59).
- **Cómo viaja:** por la red local, como hasta ahora; fuera de casa, uno comparte datos (punto de acceso) y el otro se conecta; a distancia, solo con la nube. Bluetooth directo se descarta: Tauri no tiene plugin oficial y Quick Share no llega a Linux ni a Windows.

**Consecuencias:** cambian el formato de operación en `spec/` y `crates/domain`, las tablas locales, el emparejamiento y la sincronización (Fase 4 del ROADMAP).

## D55 · Personas, roles y peticiones, también sin servidor — `aceptada`

**Decisión:**
- **La persona es el miembro** de la biblioteca (el que ya era dueño de ejemplares). Al compartir con un dispositivo se dice de quién es; una persona puede tener varios.
- **Los permisos, las lecturas, los deseos y los ejemplares son de la persona**, no del dispositivo.
- **Roles:** administrador (todo, incluidos permisos y peticiones), editor (crea y edita; borra solo lo que creó, D45) y lector (ve, marca lo suyo y pide).
- **Nadie da más de lo que tiene. Compartir es un permiso aparte**, que no se tiene por defecto: un lector puede enseñar la biblioteca en su pantalla, pero no compartirla. Puede haber **varios administradores**.
- **Peticiones:** lo que un rol no puede hacer se pide («añadir este libro», «borrar este»), y los administradores aceptan o rechazan.
- **Cada dispositivo comprueba lo que recibe** contra los roles, que se sincronizan con la biblioteca. Lo que no está permitido pasa a petición, también si alguien perdió el permiso mientras estaba sin conexión. Todos los dispositivos tienen que llegar al mismo resultado aunque les lleguen las cosas en distinto orden; se diseña con casos de conformidad compartidos en `spec/`.
- Entre conocidos se confía en el reloj de cada cambio (HLC).
- **Los mismos roles con cuenta y sin ella.** En la nube, el miembro se enlaza con una cuenta (D46).

**Consecuencias:** sustituye a D40 y D44 y amplía D47. Es la Fase 5 del ROADMAP.

## D56 · Cada dispositivo firma sus operaciones — `aceptada`

> **Ampliada el 03/10/2026 (D84):** la firma es obligatoria. Una operación sin firma se rechaza en todas partes (dispositivos y nube), también sin roles: era una excepción solo para los datos de antes de la Fase 5, que son de prueba. Las instalaciones de antes del 27/09 tienen historia sin firmar que ya no viaja; para sincronizar, empezar de cero o «Duplicar como nueva» (D58).

Cada dispositivo tiene un par de claves Ed25519, firma sus operaciones y los demás las verifican con la clave pública que reciben al emparejar. Así se sabe quién hizo cada cambio aunque llegue a través de otro dispositivo, y D55 puede comprobar permisos sin servidor. Es lo que hacen SSH o Signal; nada de cadenas de bloques.

## D57 · Dos formas de usar la app; la nube es un dispositivo más — `aceptada`

**Decisión:**
- **Sin cuenta:** gratis, sin registro y sin nube; sincroniza por la red local y hace copias donde el usuario quiera. Es la opción de la privacidad y existirá siempre.
- **Con cuenta:** un botón de registro con el email (D49), y todo se guarda y sincroniza en la nube, también a distancia y sin cobertura.
- **La nube es un dispositivo más**, siempre encendido, con los mismos roles (D55). Al registrarse se sube lo que ya hay, sin perder nada.
- **La identidad es la cuenta**, no el dispositivo, que cambia al cambiar de móvil o reinstalar.
- **Un servidor que entiende los datos**, no un relé cifrado: comprueba permisos y podrá servir una web (D53) a ludotecas o colegios. La privacidad total la da el modo sin cuenta.

**Consecuencias:** matiza D05. El servidor Symfony, el panel (D51, D52) y la landing son de este mundo (Fases 7, 8 y 11 del ROADMAP).

## D58 · Copias sin claves y recuperación del control — `aceptada`

**Decisión:**
- Exportar y guardar una copia (por ejemplo, en Google Drive) tiene que ser muy fácil y estar a la vista. Sincronizar no es hacer copia: un borrado por error también se sincroniza.
- **La copia no lleva claves ni contraseña.** Si se pierde un dispositivo, el control se recupera así:

| Caso | Cómo |
|---|---|
| Con cuenta en la nube | Con el email |
| Hay otro administrador, u otro dispositivo del mismo administrador | Desde él, quita el dispositivo perdido y vincula el nuevo (el rol es de la persona, no del dispositivo, D73) |
| Biblioteca personal, de una sola persona | Al restaurar la copia pasa a ser suya con una clave nueva |
| Compartida, con un único administrador que pierde su único dispositivo | Duplicarla como nueva (ver abajo) |

- **Caso del administrador único, cerrado el 27/09/2026:**
  - **Prevenir:** la app avisa cuando la biblioteca tiene una sola persona administradora, y sugiere nombrar a otra (o, al menos, vincular otro dispositivo suyo). Criterio ajustado el 27/09/2026 con las pantallas de Design; antes era «un solo administrador con un solo dispositivo».
  - **Último recurso:** quien tenga la biblioteca la duplica como una nueva, de la que es administrador, y los demás vuelven a unirse. Se pierden el historial y los vínculos, pero no los datos.
  - **Con cuenta en la nube** (Fase 7), el control se recupera con el email.
  - **Descartado el código de recuperación:** es un secreto más que se pierde o se deja a la vista, y quien lo encuentre se queda con la biblioteca.
- **Restaurar una copia no devuelve el control** de una biblioteca compartida: el dispositivo nuevo tiene otra clave, y vincularlo a una persona es cosa de un administrador. Si bastara la copia, cualquiera con el fichero podría hacerse administrador.
- **Perder los datos** (un dispositivo que era la única copia y sin copia de seguridad) no tiene arreglo: lo evitan las copias y compartir con otro dispositivo, que ya es una copia completa. La **copia automática periódica**, como la de WhatsApp, entra en el roadmap (Fase 9, cambiado el 03/10/2026 tras la auditoría SEG-001): la app escribe su propia copia sin claves, se activa y desactiva en Ajustes, y el destino (solo local, Google Drive u otro servicio) se decide al llegar.
- **La copia de Android y el paso de un móvil a otro no llevan nada** (03/10/2026, auditoría SEG-001): la base de datos guarda la clave de firma y los secretos de emparejamiento, y Android copia ficheros enteros. `allowBackup="false"` y `data_extraction_rules.xml` lo excluyen todo. Restaurar el catálogo es cosa de las copias de la app.
- La biometría no ayuda: sus claves no salen del chip del móvil.

## D59 · Organizaciones: la puerta abierta, sin construirlas — `aceptada`

Las organizaciones (un colegio o una ludoteca con muchas bibliotecas y personas) **no aparecen en el uso normal**: compartir con la familia tiene que ser sencillo y sin nada que suene a corporativo. Se deja la puerta abierta con el propietario de la biblioteca (D54), que hoy es una persona y mañana podrá ser una organización, como en GitHub. Se construirían en la Fase 11, solo si llega ese escenario.

## D60 · Tipos de catálogo: lista cerrada y «Otro»; el alta empieza por buscar — `aceptada`

**Decisión:**
- **Tipos con ficha propia y proveedores:** libros (incluidos rol, PDF y ebooks), cómics, juegos de mesa, películas, series (desde el 28/09/2026, dos tipos), videojuegos y **música** (vinilo, CD, casete y digital; añadida el 26/09/2026, se nos había olvidado). Lista cerrada: el usuario no crea tipos.
- **«Otro»** para todo lo demás (sellos, monedas, Playmobil): los campos comunes a todo y sin proveedores. Es el mismo formulario sin la parte específica de cada tipo.
- **Formatos por tipo**, físicos y digitales. PDF y ebook son formatos de un libro, no otro tipo. Lo digital no tiene estantería: su ubicación es «Steam» o «Kindle».
- **El alta empieza por buscar o escanear, no por el ISBN**, y **siempre se puede hacer a mano**, sin buscar nada. El ISBN es un campo de los libros; el escáner lee cualquier código y la app decide qué es.
- Los campos personalizados por el usuario quedan fuera, salvo que alguien los pida de verdad.

**Consecuencias:** se apoya en D08 y D24. Es la Fase 6 del ROADMAP.

**Cambio (28/09/2026):** películas y series de televisión son **dos tipos**, porque sus formatos son distintos. Una película relacionada con una serie es un caso puntual que no se modela.

**Cambio (29/09/2026, con las pantallas de la 6.2):** la **plataforma del videojuego va en cada ejemplar**, no en la obra (un Zelda en Switch físico, en Wii U y en Switch digital); los videojuegos se quedan sin datos propios en la edición, porque el estudio es un contribuidor. En una **serie**, la obra guarda el total de temporadas y **cada ejemplar dice cuáles tiene**, en texto libre («1–2»). Al cambiar de tipo se conserva lo que tiene equivalente (la autoría pasa a guion, el ISBN a código de barras) y solo se avisa de lo que se pierde.

## D61 · Series y estados: lo tengo, me falta, lo deseo — `aceptada`

**Decisión:**
- **Serie** agrupa obras, con orden (una saga) o sin él (una línea de producto, como Catán). Una obra puede estar en varias series, y una serie puede tener obras de varios autores. Relación **«necesita»** entre obras (expansiones, suplementos).
- Editorial, autor o estudio son datos para filtrar, no para agrupar.
- **Tres estados:** «lo tengo» (tiene ejemplares; es lo que se ve por defecto), «me falta» (está en el catálogo sin ejemplares; su propia vista, y huecos en cada serie) y «lo deseo» (marca para regalos sobre algo que falta).
- **Leído y deseado son marcas por persona** (D55). Ampliado por D68: «leído» pasa a ser un estado.

**Consecuencias:** sustituye a la lista de deseos de D36; su migración se decide al construirlo (Fase 6).

**Al preparar la Fase 6 (28/09/2026):**
- **Sin migración:** la lista de deseos de D36 está vacía; se sustituye sin más.
- **«Lo deseo» es una marca** (un campo más de D68): cada persona lo pone en lo suyo sin pedir nada, sea cual sea su rol.
- **Un lector no crea obras,** tampoco las «que faltan»: por principio de los permisos, lo pide (D73). La regla de 5.7 que dejaba a un lector apuntar deseos sueltos desaparece con la entidad `wish`.

## D62 · Proveedores: todos los posibles, con las claves en el servidor — `aceptada`

**Decisión:**
- Los proveedores **ayudan y nunca son imprescindibles** (D18). Se integran todos los posibles; el catálogo de tipos es cerrado, así que cada uno se prueba a conciencia.
- **Las claves van en el servidor.** Quien monte el suyo pone sus propias claves.
- **Sin nube**, solo los proveedores sin clave (Open Library, Inventaire, Wikidata, GameUPC), más Google Books con clave propia opcional en Ajustes. La app avisa de lo que no está disponible y lo explica en una sección de ayuda.
- **Con nube**, se activan todos, porque busca el servidor.
- Tabla de proveedores y condiciones: [research/proveedores.md](research/proveedores.md).

**Consecuencias:** varios proveedores prohíben el uso comercial (BoardGameGeek, TMDB, IGDB, Comic Vine): un servicio de pago necesitaría licencias. Sustituye a D21.

**Criterio unificado (28/09/2026):** cualquier proveedor que pida clave admite la **clave propia** del usuario en Ajustes, sin nube, como ya hacía Google Books; con la nube, las pone el servidor. En la Fase 6 se construyen los proveedores sin clave y Ajustes preparado para claves de cualquier proveedor; los de clave llegan después, uno a uno.

## D63 · Distribución: Google Play, sin iOS — `aceptada`

**Decisión:**
- **Android:** APK durante las pruebas, distribuida a familia y amigos con la cuenta de público reducido de Google (hasta 20 dispositivos, sin pago ni documento de identidad); después, Google Play (25 $ una vez, prueba cerrada con 12 personas durante 14 días y verificación de identidad).
- **F-Droid:** por estudiar, según cómo quede tras la verificación de desarrolladores de Google (desde el 30/09/2026, global en 2027).
- **iOS, fuera:** exige un Mac y 99 $ al año.
- Una **landing** pública explica el proyecto y enlaza a la tienda y al código.

## D64 · La documentación de trabajo, fuera de la versión pública — `abierta`

**Planteamiento del mantenedor (25/09/2026):** la documentación de trabajo (decisiones, planes, debates, investigación) no debería estar en la versión pública del repositorio: nadie tiene que ver sus notas mentales.

**Lo que complica:** D03 contaba con que abrir el repo fuera «cambiar un interruptor». Pero el historial de git guarda todo lo que ha pasado por `docs/`: aunque se borre la carpeta antes de publicar, sigue en los commits antiguos.

**Opciones a debatir:**

| Opción | Cómo | A favor | En contra |
|---|---|---|---|
| Repositorio privado aparte para el trabajo | `docs/` de trabajo se mueve a otro repo privado (por ejemplo, junto a este); el público solo lleva código, `spec/` y documentación para usuarios | Separación limpia desde ya | Los agentes y el código dejan de tener las decisiones al lado; hay que enlazar dos repos |
| Publicar con historial nuevo | Al abrirlo, el público empieza con un primer commit limpio; el repo privado actual sigue siendo el de trabajo | Nada que tocar hasta entonces | Dos repos que mantener sincronizados a partir de ese día |
| Limpiar el historial antes de publicar | `git filter-repo` quita `docs/` de todos los commits | Un solo repo | Reescribe todo el historial; hay que hacerlo con cuidado y con copia |

**Argumento nuevo (26/09/2026):** además de la documentación de trabajo, el historial guardaría el renombrado desde «library» (D70). Refuerza la opción «Publicar con historial nuevo»: el público nacería con un primer commit limpio en `kosmotheka/kosmotheka`, que queda reservado para eso, y el privado actual seguiría siendo el de trabajo. Sigue abierta: se decidirá al debatirla con el maintainer.

Relacionada con D03 y con la documentación pública futura (guías de usuario e instalación, D12, D22).

## D65 · Borrar una biblioteca y salir de ella — `aceptada`

**Contexto:** con varias bibliotecas (D54) hace falta decidir qué significa quitar una: borrarla para todos o solo dejar de tenerla. Salió al revisar el borrador de casos de conformidad de la Fase 4.

**Decisión:**
- **Borrar una biblioteca** la quita, con todo lo que contiene, **en todos los dispositivos** que la tienen. Es una operación más (un `delete` de la entidad `library`) y solo puede hacerla quien tenga permiso para ello (D55).
- **Antes de borrar, la app pide confirmación**, avisa de que **no se puede deshacer** (tampoco en los demás dispositivos) y **ofrece hacer una copia de seguridad** de esa biblioteca en ese mismo momento.
- **Salir de una biblioteca** es otra cosa: la biblioteca sigue para los demás y este dispositivo (y su persona) deja de tenerla.
  - Quien **no es administrador** puede salir siempre.
  - Un **administrador** puede salir solo si **no es el último**; si lo es, antes tiene que nombrar a otro o borrar la biblioteca.
- **Por valorar:** mostrar quién creó cada biblioteca, para saber de dónde viene. Se puede sacar de la operación que la creó (el dispositivo hoy, la persona con la Fase 5) sin añadir un campo.

**Consecuencias:** el borrador de `spec/` recoge el borrado (caso «library deleted» y regla en `spec/README.md`). Los permisos para borrar y las reglas de salida se concretan con los roles en la Fase 5; hasta entonces, sin roles, cualquier dispositivo con la biblioteca puede borrarla, como pasa hoy con todo lo demás. Amplía D54 y D55.

## D66 · Registro de actividad para poder responder de lo que pasó — `aceptada`

**Contexto:** si alguien borra una biblioteca, o echa a alguien de ella, hay que poder decir **quién fue, cuándo y desde dónde**, para responder ante una reclamación. Sobre todo en la nube, que es un servicio para terceros, aunque sea gratuito.

**Qué ya tenemos:** el registro de operaciones (D25) guarda cada cambio con su dispositivo y su hora, y con D56 cada operación irá firmada por su dispositivo. Pero no basta: al borrar una biblioteca, su registro se va con ella, y la hora de una operación la pone el reloj del dispositivo, que puede estar mal.

**Decisión:**
- **En la nube, un registro de actividad aparte**, solo de añadir, que **sobrevive a lo que registra**: si se borra una biblioteca, su entrada sigue. Cada entrada guarda:
  - **qué:** borrar o crear una biblioteca, salir o echar a alguien, cambiar un rol o el permiso de compartir, emparejar o revocar un dispositivo, iniciar sesión, cambiar la contraseña, borrar una cuenta, y los accesos del superadministrador (D52);
  - **quién:** la cuenta y el dispositivo;
  - **cuándo:** la hora **del servidor**, no la del dispositivo;
  - **desde dónde:** la dirección IP y el agente (app y versión);
  - **sobre qué:** la biblioteca, con su nombre en ese momento, para que se entienda aunque ya no exista.
- **Lo ve el superadministrador** en el panel (D52). Los administradores de una biblioteca verán lo de la suya en su historial (Fase 5).
- **En local**, sin nube, cada dispositivo **conserva la operación de borrado** de una biblioteca (quién, cuándo, qué biblioteca) aunque borre todo su contenido. Sin servidor no hay IP ni hora de confianza: es lo que hay, y basta para uso doméstico.
- **Se guarda un año** (aceptado el 01/10/2026) y después se borra solo. La IP es un dato personal: se guarda por seguridad, con ese plazo, y la política de privacidad lo dice.

**Consecuencias:** amplía D52 (su tabla de eventos para los KPI puede ser la misma) y D65. La parte local entra en la Fase 4, con el borrado de bibliotecas; la de la nube, en la Fase 7.

## D67 · Enlaces para compartir y etiquetas QR: son de la biblioteca — `aceptada`

**Contexto:** al hablar de mover un libro entre bibliotecas (D46) salió qué pasa con un enlace para compartir un ejemplar, o con la etiqueta QR pegada en él, cuando el ejemplar se muda. Debate completo en [research/referentes.md §3.4](research/referentes.md).

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| Un código del ejemplar que viaja con él al moverlo | El enlace y la etiqueta siguen valiendo | Mezcla la identidad del ejemplar con el permiso para verlo, y sería una identidad que cruza bibliotecas |
| **El enlace y la etiqueta son filas de la biblioteca de origen** | Un permiso dado en una biblioteca nunca abre otra; sin excepciones al mover | Al mover hay que compartir otra vez e imprimir otra etiqueta; mover es raro |

**Decisión** (acordada el 26/09/2026):
- **Un enlace para compartir un ejemplar es un permiso**, dado desde una biblioteca concreta, no una dirección del libro. Es una fila de esa biblioteca y **muere con el ejemplar** al moverlo o borrarlo; quien lo abra ve «ya no está disponible». Si sobreviviera a la mudanza, un permiso dado en la biblioteca personal abriría una puerta en la familiar sin que la familia lo decidiera.
- **El enlace lleva un token aleatorio** generado al compartir, independiente del id (ni el UUID ni un hash suyo): se puede revocar sin tocar el libro, puede haber varios por ejemplar con su propia caducidad y no revela la fecha que lleva un UUID v7. Mismo modelo que las invitaciones (D43).
- **La etiqueta QR impresa también es de la biblioteca**: al mover el ejemplar deja de valer y se imprime otra. Lleva un **código corto aleatorio**, no el UUID: el QR sale más pequeño y no revela nada.
- Se mantiene D46: mover crea ids nuevos, porque las lápidas del origen (D32) impedirían que el mismo id volviera.

**Consecuencias:** no cambia nada de la Fase 4: enlaces y etiquetas aún no existen. Cuando lleguen serán filas de la biblioteca que se borran con el ejemplar, sin lógica especial al mover. Relacionada con D43, D46 y D54.

## D68 · Marcas por persona: estado, valoración y favorito — `aceptada`

**Contexto:** al revisar otras apps de catálogo (acordado el 26/09/2026, [research/referentes.md §3.2 y §3.5](research/referentes.md)), «leído» a secas se quedaba corto, y apuntar el progreso resultaba un peñazo.

**Decisión:** cada persona (D55) apunta sobre cada obra:
- **Estado absoluto**, con fecha y **sin progreso** (nada de «voy por la página 120»): pendiente, en curso, terminado o abandonado. «En curso» es **un solo estado** que la interfaz nombra según el tipo: leyendo (libros y cómics), jugando (videojuegos), viendo (películas y series). En música tiene menos sentido, pero no estorba.
- **Valoración** («este me ha encantado»).
- **Favorito**, aparte de la valoración: sirve de filtro, y un favorito puede tener menos puntuación que otros.
- **Deseado** se mantiene (D61).

**Consecuencias:** sustituye a «leído» en las marcas por persona de D61. ~~Entra en la Fase 6, junto con géneros, temáticas y etiquetas.~~

**Cambio de fase (27/09/2026):** se adelanta a la Fase 5 (hito 5.8), porque la regla que la sostiene es de permisos (cada persona marca lo suyo, sea cual sea su rol) y el objetivo de la fase ya dice «mi hijo marca lo leído». En la 5 llegan los datos, la regla y la interfaz para libros; en la 6, los nombres de «en curso» según el tipo (jugando, viendo) y los filtros por tipo.

**Al construirlo (27/09/2026, aceptado por el maintainer):**
- **Excepción a los ids UUID v7:** el id de una marca no es aleatorio, se deriva de la persona y la obra (UUID v5 de `<member_id>:<work_id>`, con un espacio de nombres fijo en `spec/README.md`). Con un v7, si la misma persona marcara la misma obra en dos dispositivos sin conexión, habría dos marcas suyas y haría falta una regla nueva para juntarlas; con el id derivado son la misma fila y se fusionan campo a campo, como todo lo demás. Hay precedentes de filas sin id aleatorio: los datos de libro usan el de su edición, una petición resuelta el de su operación y un dispositivo el suyo propio. La validación exige el id derivado (`wrong_id`), en Rust y en PHP.
- **Las marcas no se borran con un cambio** (`undeletable`): una lápida en su id derivado impediría a esa persona volver a marcar la obra nunca. Vaciar una marca deja sus campos a `null`; la marca solo se va, en cascada, con su obra o con su persona.
- **Cada persona marca lo suyo, sea cual sea su rol;** quien administra, las de todos. Un dispositivo sin persona no marca. En una biblioteca sin roles, el dispositivo dice antes quién es («¿Quién eres?»).
- **Valoración de 1 a 5**, sin medias estrellas.
- **Todas las personas ven las marcas de las demás** («Lo de los demás» en la ficha), haya roles o no: en casa se ve quién ha leído qué, como el antiguo «Lo han leído». Ver no es un permiso: cada dispositivo tiene la biblioteca entera. Lo que sí es un permiso es cambiarlas (cada persona las suyas; quien administra, todas).

## D69 · Reserva de regalos oculta solo en pantalla — `aceptada`

**Contexto:** la reserva de regalos (Fase 9) tiene que ser una sorpresa. Pero en una biblioteca compartida cada dispositivo la tiene entera (D54), incluido el de quien recibe el regalo, así que sin servidor el dato no se puede ocultar de verdad ([research/referentes.md §3.4 y §3.6](research/referentes.md)).

**Decisión** (acordada el 26/09/2026): se mantiene, **oculta solo en pantalla**. El deseo lo ve toda la familia; la reserva («lo compro yo») la ven todos menos quien recibe el regalo. En familia basta para una sorpresa. **Con la nube se hace igual**, para que las dos formas de usar la app (D57) se comporten del mismo modo.

**Consecuencias:** no hace falta cifrar ni separar datos por persona. Fase 9.

## D70 · Nombre del proyecto: Kosmotheka — `aceptada`

**Contexto:** «library» era un nombre de trabajo. Hace falta uno propio antes de publicar en Google Play (Fase 8) y de abrir el repositorio. Toda la búsqueda está en [research/naming.md](research/naming.md).

**Opciones finales:**

| Opción | A favor | En contra |
|---|---|---|
| **Kosmotheka** | Griego *kosmos* (mundo ordenado) + *theka* (depósito): el cosmos de tus cosas. Sirve para cualquier colección, no solo libros. .com y .app libres, Google Play y marcas limpios, `kosmotheka` libre en GitHub | Sus errores de escritura más probables (Kosmoteka, Kosmotek) tienen el .com ajeno |
| Pantheka | También con .com, .app, Google Play y marcas limpios | En español se pronuncia «panteka», cerca de marcas de panadería |

**Decisión** (26/09/2026): **Kosmotheka**, con k y th. Pantheka queda como segunda opción. El maintainer registra kosmotheka.com y kosmotheka.app.

**Consecuencias:**
- Solo cambia el nombre del **producto**. En el código, `library` sigue siendo la entidad biblioteca (D38, D46, D54) y no se renombra.
- El renombrado se hace por etapas según [plans/renombrado-kosmotheka.md](plans/renombrado-kosmotheka.md). Lo primero es el identificador de la app, porque hay que reservarlo en Google Play con una primera versión en prueba interna, y cambiarlo después sería otra app.

## D71 · Dejar de compartir quita el acceso y borra la biblioteca en el otro dispositivo — `aceptada`

**Contexto:** en la Fase 4 cada biblioteca se comparte con dispositivos concretos (hito 4.4). Hay que decidir qué pasa cuando alguien deja de compartir una biblioteca con un dispositivo. El caso que manda: dos personas se enfadan y una no quiere que la otra vea nada de lo suyo, ni siquiera una copia, ni que pueda tocarlo.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| A. El otro deja de recibir cambios y conserva lo que tenía | No se pierde nada | El otro se queda con una copia completa: justo lo que no se quiere |
| B. Se le quita el acceso y la biblioteca se borra en su dispositivo | Hace lo que se pide: retira el acceso de verdad | Borra datos en un dispositivo ajeno |

**Decisión (B):**
- **Dejar de compartir es retirar el acceso.** El otro dispositivo **borra esa biblioteca** (su contenido y su registro, como al salir, D65) y deja de recibirla.
- **Nada de lo que haga después le llega a la biblioteca:** sus cambios posteriores se rechazan.
- **Se puede volver a dar acceso** emparejando otra vez; entonces recibe la biblioteca entera de nuevo.
- Lo hace quien tenga permiso para ello; sin roles (hasta la Fase 5), cualquier dispositivo que tenga la biblioteca, como con todo lo demás (D65).
- Queda en el registro de actividad (D66): quién retiró el acceso, a qué dispositivo y cuándo.

**Propuesta técnica (añadida por Claude, a revisar al construirla):**
- **La lista de dispositivos revocados es de la biblioteca**, no de un dispositivo: se guarda como una operación más y viaja a todos los que la comparten. Así ninguno de mis otros dispositivos le sigue pasando datos por la puerta de atrás (el portátil deja de compartir, pero la tableta seguía emparejada con ese móvil).
- **Cómo se entera el otro:** la próxima vez que conecte con cualquiera de mis dispositivos, recibe «acceso retirado a esta biblioteca» y la borra.
- **Sus cambios:** se aceptan los anteriores a la retirada y se rechazan los que lleguen firmados por ese dispositivo después.

**Límites, dichos claramente:**
- **Sin servidor, el borrado ocurre cuando el otro vuelve a conectar.** Si no conecta nunca, conserva lo que tenía hasta entonces, aunque sin recibir nada nuevo. Con la nube (Fase 7) la retirada es inmediata para todo lo que pase por ella.
- **No se puede borrar lo que ya haya sacado fuera:** una copia de seguridad o un CSV exportados antes, capturas de pantalla, o una app modificada que ignore la orden. Eso no lo evita ninguna aplicación; lo que sí se garantiza es que no recibe nada nuevo y no puede cambiar nada.

**Cómo quedó al construirlo (hito 4.4):** detalle en [architecture/sync.md](architecture/sync.md#compartir-y-retirar-el-acceso-hito-44-d71).
- La propuesta técnica se mantiene. La retirada es la entidad `revocation`; los avisos viajan en cada sincronización.
- **Añadido:** cada dispositivo elige lo que envía, y una biblioteca que el otro ofrece se acepta sola **solo si no la tiene ya**. Una que los dos tienen (como «Mi biblioteca», con el mismo id en todos por D46) solo se une si los dos la marcan.
- **Añadido:** salir de una biblioteca avisa a los demás para que dejen de ofrecerla.
- **Límite más:** los dispositivos que la recibieron **a través** del retirado (otros suyos, que nunca se emparejaron con los tuyos) no se conocen y la conservan. Con cuentas (Fase 5) la retirada será por persona y los alcanzará.
- **Sin roles todavía,** cualquier dispositivo que la tenga puede retirar el acceso a cualquier otro, también al que la creó (como con borrar, D65).

**Consecuencias:** concreta el hito 4.4. Casos de validación de `revocation` en `spec/`; tests por TLS real en `lan/client.rs`, uno de ellos con tres dispositivos. Amplía D65 y D66; la parte con cuentas y roles se retoma en la Fase 5 (D55).

## D72 · La marca (nombre y logo), fuera de la AGPL y protegida — `propuesta`

**Contexto:** hace falta un logo para Google Play y la web ([encargo](design/encargo-logo.md), ya pasado a Claude Design). El logo tiene que ser original y estar protegido. Propuesta preparada en la sesión del logo con el maintainer.

**Propuesta:**
1. **El nombre y el logo quedan fuera de la AGPL (D11).** El código es libre, pero la marca no. Una política de marca aparte dice:
   - qué se puede hacer: hablar del proyecto, enlazarlo, usar el logo para referirse a él;
   - qué no: usarlo en un servicio o una versión modificada que se haga pasar por la oficial.

   Lo hacen Mozilla con Firefox y la Rust Foundation con Rust. Encaja con D04: el servicio alojado es el «oficial».
2. **Comprobar que el logo está libre antes de elegir el definitivo:**
   - búsqueda por imagen en [TMview](https://www.tmdn.org/tmview/) (UE y oficinas nacionales, incluida la OEPM, y otras) y en la [WIPO Global Brand Database](https://www.wipo.int/en/web/global-brand-database) (unos 38 millones de marcas, con búsqueda de conceptos por IA);
   - búsqueda inversa (Google Lens, TinEye) para logos no registrados;
   - lo que cuenta es el parecido en las clases de Niza 9 (software) y 42 (servicios en la nube). Guía de la EUIPO: [antes de solicitar](https://www.euipo.europa.eu/en/trade-marks/before-applying/availability).
3. **Registrar el logo como marca figurativa**, no solo el nombre. Un logo hecho con IA (Claude Design) puede no tener derechos de autor: en EE. UU. no se protege lo que no tiene autoría humana, y en la UE se exige una creación intelectual humana. Retocarlo a mano ayuda, pero la protección fiable es el registro.
4. **Por valorar:** un informe de búsqueda de un agente de la propiedad industrial antes de registrarlo.

**Consecuencias:**
- Antes de abrir el repositorio hay que añadir la exclusión en LICENSE y README, y un TRADEMARKS.md o similar. Relacionado con D64 y con el repositorio kosmotheka/kosmotheka.
- En la Fase 8 del ROADMAP: la búsqueda de marcas y el registro del logo, junto al del nombre si lo hay.
- Amplía D11 y D70.

## D73 · Cómo se comprueban los permisos, sin servidor y con él — `aceptada`

**Contexto:** D55 decide que cada dispositivo compruebe lo que recibe contra los roles, y que todos lleguen al mismo resultado sea cual sea el orden de llegada. Faltaban las reglas concretas. Se propusieron en el plan de la Fase 5 y en los casos en borrador `spec/conformance/draft/permissions.json` (reglas en su README), y el maintainer las aceptó el 27/09/2026.

**Decisión:**
- **Sin roles, como hoy:** una biblioteca sin ningún rol asignado deja hacer todo a todos. El primer rol se asigna con esa misma regla, y quien lo asigna se hace administrador.
- **Quién y con qué rol:** el autor de una operación es el dispositivo de su `hlc` (firmado, D56); su persona, el miembro al que apunta su fila `device`; su rol, el de ese miembro **en el momento de la operación**. Un dispositivo que no es de nadie cuenta como lector.
- **Roles:** un campo `role` en el miembro (`admin`, `editor`, `reader`) y otro, `can_share`, para compartir. Cambiar roles, el permiso de compartir o de quién es un dispositivo es cosa de administradores. El editor borra solo lo que creó su persona (D45).
- **Cada dispositivo se registra y se renombra a sí mismo**, tenga el rol que tenga.
- **Todo o nada:** si un solo cambio de una operación no está permitido, la operación entera pasa a petición.
- **Converger:** cada operación se juzga con los roles vigentes en su HLC. Si llega tarde una que cambia roles, el dispositivo recalcula la biblioteca desde su registro.
- **Nada se pierde:** lo no permitido se queda en el registro, viaja como las demás y es una petición para los administradores.
- ~~**Operaciones sin firma** (anteriores a la Fase 5): se aceptan mientras la biblioteca no tenga roles; con roles, cuentan como de un lector.~~ **Cambiado el 03/10/2026 (D84):** toda operación va firmada; una sin firma se rechaza en todos los dispositivos y en la nube.

**Añadido al revisar las pantallas de Design (27/09/2026):**
- **Activar los roles incluye decir de quién es cada dispositivo.** Quien activa los roles pasa a administrar; en el mismo paso asigna persona a los dispositivos que ya comparten la biblioteca, con **editor** por defecto. Así nadie pierde de golpe lo que podía hacer, y se mantiene la regla de que un dispositivo sin persona cuenta como lector (un dispositivo perdido y quitado no conserva permisos). Todo va en una sola operación, que se juzga cuando aún no hay roles.
- **La persona es la misma en todas las bibliotecas:** al añadir a alguien a otra biblioteca se reutiliza su id de miembro (los ids solo tienen que ser únicos dentro de cada biblioteca). La interfaz agrupa por ese id. No cambia el formato de las operaciones.
- **Peticiones:** nota opcional de quien pide, «Editar y aceptar» (el administrador corrige antes de aceptar), retirar una petición propia pendiente, y quién la resolvió y cuándo.
- **Aviso de administrador único:** siempre que la biblioteca tenga una sola persona administradora, tenga los dispositivos que tenga (el texto dice cuáles perdería).

**Añadido con la segunda revisión de pantallas (27/09/2026):**
- **Un rol nunca se hereda:** lo da, activamente, alguien con permiso para ello. Al emparejar un dispositivo de alguien que ya está en otras bibliotecas, en las que aún no tiene rol hay que elegirlo; en las que ya lo tiene, se mantiene.
- **En móvil, emparejar va en dos pasos:** «¿De quién es?» y, solo si falta algún rol, «Qué recibirá y con qué rol». En escritorio cabe en una pantalla.
- **Los roles se nombran en forma neutra** en español (Administrador, Editor, Lector): la app no sabe el género de cada persona.

**Añadido al construir «Pedir en lugar de hacer» (27/09/2026):**
- **Quien lee apunta deseos:** un deseo no cambia nada de las estanterías, así que un lector con persona puede crear deseos en «Me falta» y editar o borrar los que añadió su persona. Un dispositivo sin persona, o una operación sin firma, sigue sin poder: cualquiera podría hacerse pasar por él. Los casos de conformidad que usaban «un lector añade un deseo» como ejemplo de petición pasan a usar una editorial, y hay un caso nuevo para la lista de deseos.
- **«Puede compartir» se comprueba en el dispositivo que comparte:** en una biblioteca con roles, solo quien administra o tiene «Puede compartir» la lleva a otro dispositivo; sin roles, cualquiera, como antes. Quien no puede no ve la opción (al emparejar ni en dispositivos, donde sale «En Familia no puedes compartir; pídeselo a quien la administra») y el motor lo rechaza igualmente (`people::may_share`). **Límite aceptado:** compartir no es una operación que juzguen los demás, sino entregar datos que el dispositivo ya tiene, así que una app modificada podría entregarlos igual. El dispositivo que los recibe no es de nadie: cuenta como lector, solo lee y pide, y quien administra lo ve «sin persona» y le puede retirar el acceso. Se descartó que los demás dispositivos rechacen a quien trajo alguien sin permiso: cambiaría el formato del alta y aun así no impediría la lectura.

**Consecuencias:** los casos pasan de `draft/` a `conformance/` cuando se construyan (hitos 5.3 y 5.4 de `docs/historico/plan-fase-5.md`), con un tipo de campo booleano nuevo para `can_share`. Concreta D55 y D56.

**Añadido al preparar la Fase 7 (01/10/2026): la nube juzga como un dispositivo más.** Sustituye a D47.
- La nube aplica estas mismas reglas a cada operación que recibe: con el rol de su autor **en el momento de la operación**. Lo de un administrador se guarda y se aplica tal cual; lo de un lector, o lo que alguien hizo sin conexión después de perder el permiso, queda como petición, en la nube y en todos los dispositivos.
- **La nube solo rechaza lo que rechazaría cualquier dispositivo:** una operación con formato o versión de esquema incorrectos, con una firma que no cuadra (alguien que se hace pasar por otro), o de un dispositivo al que se le retiró el acceso (D71). Lo rechazado no se reenvía.
- **La nube nunca corrige a un dispositivo.** El caso que motivó D47 (un editor sin conexión borra un libro suyo mientras le bajan a lector) se resuelve solo: al llegarle el cambio de rol, más antiguo que su borrado, su dispositivo recalcula la biblioteca, el libro vuelve y el borrado queda como petición que se puede aceptar. Ninguna lápida se levanta por orden de nadie.
- **Escenarios revisados con el maintainer:** administradora que edita sin cobertura (se aplica), lector que pide borrar (petición), editor que pierde el rol sin conexión (petición; su dispositivo se corrige al recalcular), app modificada que hace lo que su rol no permite (petición), operación falsificada (rechazada por la firma) y dispositivo retirado que sigue enviando (rechazado).
- **Hora y zonas horarias:** la marca HLC son milisegundos desde 1970 en UTC, así que las zonas horarias no influyen; solo se pasan a hora local para mostrarlas. El límite que sigue es el de D55: un dispositivo con el reloj mal puesto, o manipulado, fecha mal sus cambios. Con cobertura, la nube no tiene prioridad sobre nadie: gana el cambio más reciente campo a campo, como siempre. La nube guarda además la hora real de llegada de cada operación (D66) y puede avisar al dispositivo cuyo reloj vaya muy desviado; no reescribe marcas, porque están firmadas y también viajan por la red local.

## D74 · Publicar versiones: escritorio en GitHub Releases, Android a Google Play, ambos automáticos — `propuesta`

**Contexto:** la landing (D22) tendrá una página de descargas: Linux (`.deb`, `.rpm`, AppImage), el instalador de Windows y el enlace a Google Play, y conviene ofrecer también las versiones anteriores. Hay que decidir dónde viven los binarios y cómo llegan allí. Propuesto el 01/10/2026, al generar los primeros instaladores.

**Opciones para los binarios de escritorio:**

| Opción | A favor | En contra |
|---|---|---|
| En el repositorio, y se despliegan con la web | Todo en un sitio | Git guarda cada binario para siempre (el AppImage pesa 77 MB): el repositorio crecería sin remedio y limpiarlo obliga a reescribir la historia |
| En el VPS (`public/` o `var/downloads` de Symfony) | Enlaces propios | Subida a mano en cada versión; disco y ancho de banda del VPS; las versiones anteriores hay que gestionarlas aparte |
| **GitHub Releases** del repositorio público | Gratis, sin límite de tráfico práctico, versiones anteriores con sus notas, cuenta descargas, se llena desde el CI | Depende de GitHub |

**Propuesta:**
1. **Escritorio en GitHub Releases.** Al subir una etiqueta (`v0.2.0`), un workflow compila Linux y Windows con la acción oficial de Tauri y crea un *release* en borrador con los instaladores y sus sumas de comprobación. El maintainer lo revisa y lo publica.
2. **Nombres sin versión** en los ficheros del *release* (`Kosmotheka-linux-x86_64.AppImage`, `Kosmotheka-windows-x64-setup.exe`…), para que la landing use los enlaces fijos `…/releases/latest/download/<fichero>`, que siempre dan la última versión. Las anteriores, en la página de *releases*.
3. **Rutas propias que redirigen:** si la landing va en Symfony, `/download/{plataforma}` responde con un 302 al fichero de GitHub. Los enlaces compartidos no se rompen si un día cambia el alojamiento, y se pueden contar descargas sin cookies. En una landing estática, enlaces directos.
4. **Android, a Google Play desde el mismo workflow:** compila el `.aab` firmado con la clave de subida y lo sube a la pista de prueba (interna o cerrada) con la API de Google Play Developer. Pasarlo a producción sigue siendo un paso a mano en la consola de Play.
   - La **primera subida** es a mano: la API solo trabaja con apps que ya existen en la consola.
   - **Secretos** en GitHub, nunca en el repositorio (D03): la clave de subida (keystore en base64 y sus contraseñas) y el JSON de una cuenta de servicio de Google Cloud con permiso solo para esta app.
   - Con **Play App Signing**, Google guarda la clave que firma la app; nosotros solo la de subida, que Google puede cambiar si se pierde. Aun así, la clave de subida va con copia de seguridad fuera del repositorio.
   - El `versionCode` de Android sale de la versión de `tauri.conf.json`, así que cada versión publicada tiene que subir de número.

5. **Linux, compilado en la versión más antigua que se soporta** (añadido el 02/10/2026, al ver que la compilación hecha en el contenedor de desarrollo, Ubuntu 24.04, no se instala en el Ubuntu 20.04 del maintainer). Lo compilado en una versión de Ubuntu funciona en las posteriores, nunca en las anteriores (por la versión de glibc). Así:
   - **`.deb`, `.rpm` y AppImage, compilados en Ubuntu 22.04** en el CI (`ubuntu-22.04`), nunca en la máquina de desarrollo. Valen para Ubuntu 22.04 y 24.04, Debian 12, Linux Mint 21 y 22 y parecidas.
   - **Ubuntu 20.04 no puede tener `.deb`:** Tauri 2 necesita WebKitGTK 4.1, que Ubuntu nunca publicó para la 20.04. Le llega con la **Snap Store**, que lleva WebKit dentro: en Ubuntu aparece en el centro de software, se instala con un clic y se actualiza sola. Snapcraft puede compilarla desde GitHub, sin instalar nada en la máquina del maintainer; necesita el repositorio público.
   - **Flathub**, después, para el resto de distribuciones (Fedora, Arch…), con la misma idea.
   - **La página de descargas dice qué sistemas admite cada fichero** (por ejemplo «Ubuntu 22.04 o posterior, Debian 12…»), y recomienda la tienda (Snap Store, Flathub) cuando la haya, porque es lo más fácil para quien instala.
   - Todo sale del mismo workflow, como artefactos del *release*: todas las versiones descargables en el mismo sitio.

**Por estudiar cuando llegue:**
- **Actualizaciones automáticas en escritorio** con el plugin `updater` de Tauri: lee un `latest.json` del mismo *release*. Los ficheros van firmados con otra clave propia, también fuera del repositorio.
- **Firma de código en Windows:** sin ella, SmartScreen avisa al instalar («Windows protegió su PC»). Los certificados cuestan dinero; hay opciones gratuitas para open source (SignPath Foundation).
- F-Droid para Android (ya en la Fase 8).

**Consecuencias:**
- Concreta la página de descargas de D22, que sigue abierta en si la landing va en Symfony o estática; esta propuesta sirve para las dos.
- Necesita el repositorio público (D03, D64): los *releases* de un repositorio privado no se pueden descargar sin cuenta.
- Hoy el CI genera el instalador de Windows como artefacto de cada push (caduca a los 14 días); es solo para pruebas y no sustituye a los *releases*.
- En la Fase 8 del ROADMAP: el workflow de publicación y la página de descargas.
- Mientras tanto, en la máquina del maintainer (Ubuntu 20.04) la app se ejecuta dentro del contenedor `library-dev`; ver [development/setup.md](development/setup.md).

## D75 · Avisos al momento: Mercure con la app abierta, notificaciones push en Android — `aceptada`

**Contexto:** al preparar la Fase 7 se propuso que la app abierta preguntara a la nube cada minuto si había algo nuevo. El maintainer lo descartó (01/10/2026): preguntar cada cierto tiempo sirve para probar, pero una app moderna recibe avisos del servidor, como WhatsApp o Telegram.

**Decisión:**
- **Con la app abierta** (escritorio y móvil), una conexión abierta con el servidor que avisa «hay algo nuevo en esta biblioteca», sin datos; la app entonces sincroniza por el protocolo de siempre (D39). En el servidor, **Mercure** (el estándar de Symfony; un contenedor más, porque PHP-FPM no mantiene conexiones abiertas). Hito 7.3.
- **Con la app cerrada, en Android, notificaciones push** con FCM (Firebase Cloud Messaging), y UnifiedPush como alternativa sin Google. En la **Fase 8**, junto a Google Play. Tauri no tiene plugin oficial de push (el oficial solo hace notificaciones locales); hay varios de la comunidad.
- **Avisos con la app cerrada, para empezar:** una petición nueva (para quien administra), una petición propia aceptada o rechazada, y alguien que acepta tu invitación. Los cambios normales del catálogo no avisan, solo se sincronizan. Se añadirán más si hacen falta.
- **Lo que pasa por Google es genérico** («Tienes una petición nueva»), nunca el contenido de la biblioteca; se cuenta en la política de privacidad.
- **En escritorio, con la app cerrada, nada:** sincroniza al abrirse.
- **Red local:** tras un cambio, el dispositivo lo envía al momento a los emparejados que respondan.
- **Siempre opcional** (D18): sin Mercure ni push (por ejemplo, un servidor propio que no los monte), la app sincroniza al abrirse, al volver a primer plano y tras cada cambio.

**Opciones descartadas:** preguntar cada minuto (gasta batería y datos, y no avisa con la app cerrada); WebSockets servidos por PHP (PHP-FPM no está hecho para conexiones largas).

**Consecuencias:** un servicio más en el `docker-compose` de producción (D48) y en la guía de autoalojamiento; en la app, un cliente en Rust que escucha y reconecta, un evento de Tauri que refresca la interfaz, un indicador de conexión y, en la Fase 8, el permiso de notificaciones de Android 13 y Ajustes de avisos. Amplía D39 y D57.

## D76 · La cuenta en la biblioteca: enlace automático y la nube vincula los dispositivos nuevos — `aceptada`

**Contexto:** con D73, un dispositivo tiene el rol de una persona solo cuando un administrador dice «este móvil es de Laura» desde otro dispositivo. Con cuenta (D57) eso falla en dos casos: el móvil nuevo de alguien, que no puede hacer nada hasta que lo vinculen a mano, y la administradora única que pierde su único móvil, que no podría recuperar el control con el email como promete D58. Acordado con el maintainer el 01/10/2026, al preparar la Fase 7.

**Decisión:**
- **Subir es por biblioteca.** Registrarse no sube nada: al iniciar sesión, la app pregunta qué bibliotecas subir (D54, nada sale sin decirlo). Subir es compartir, así que lo hace quien administra o tiene «Puede compartir» (D73); una biblioteca ajena en la que solo estás no se sube por estar tú en ella.
- **El enlace entre cuenta y persona es automático cuando el dispositivo ya es de esa persona.** Si un dispositivo que inicia sesión ya es «de Sergio» en una biblioteca que está en la nube (lo decidió un administrador, D73), la cuenta queda enlazada con Sergio en esa biblioteca, sin invitación ni pregunta. Así, quien ya estaba en la familia por la red local entra en la nube con solo registrarse. Un dispositivo que no es de nadie no enlaza nada; uno que la biblioteca tiene como de otra persona hace que la app pregunte en lugar de enlazar.
- **La nube vincula los dispositivos nuevos de una cuenta enlazada.** Al subir una biblioteca, la nube se registra en ella como un dispositivo más, con su clave, y quien administra le concede, con una operación firmada, un único poder: decir que un dispositivo de una cuenta enlazada es de esa persona. No cambia roles ni toca el catálogo. Es una casilla marcada por defecto al subir, que se puede quitar en los ajustes de la biblioteca; sin ella, cada dispositivo nuevo se vincula a mano, como hoy.
- **Escenarios:** móvil nuevo (inicia sesión y recibe cada biblioteca de la nube en la que está su cuenta, con su rol); administradora única sin móvil (recupera el control con su email, D58); contraseña robada (el intruso entraría con ese rol: cambiar la contraseña cierra los demás dispositivos, D50, y un administrador quita el del intruso, D58); biblioteca que nunca se subió (nada cambia).
- **Lo que concede, dicho claro:** la cuenta vale tanto como el rol de su persona. Es lo normal en un servicio con cuenta, y lo que dice D57: la identidad es la cuenta.
- **Lo que no está en la nube no baja de la nube:** en un móvil nuevo vuelve por la red local, desde otro dispositivo que la tenga, o desde una copia.

**Dónde se guarda el enlace** (acordado el mismo día):
- **En el servidor**, con los usuarios al estilo de Symfony: el email y la contraseña (o Google, D77) solo existen en la nube, y una tabla suya dice «esta cuenta es esta persona en esta biblioteca». El rol no se copia ahí: se lee siempre de la persona, así que no hay dos verdades.
- **Con la biblioteca viaja solo que la persona tiene cuenta**, en un campo nuevo de `member` (`account_id`) que solo usa la nube. Sirve para que cualquier dispositivo, también sin conexión o por la red local, sepa quién tiene cuenta (por ejemplo, para no invitar a quien ya está). El email no viaja nunca.
- **Detalles, confirmados por el maintainer:** el valor es un id aleatorio de la cuenta, no un hash del email (un hash de email se adivina probando emails conocidos); solo lo escribe la nube, con su clave, al enlazar o desenlazar; y es solo informativo: quién puede entrar lo decide siempre la tabla del servidor, así que un valor falso no da nada.

**Por concretar al construirlo (hito 7.5):** cómo se marca en `spec/` el poder de la nube (una regla nueva de permisos con sus casos de conformidad, en Rust y en PHP). El campo `account_id` cambia a la vez la migración, `schema.rs`, `spec/` y `CatalogEntity.php`.

**Consecuencias:** cumple la recuperación con el email de D58. Amplía D57 y D73.

## D77 · Entrar con email o con Google — `aceptada`

**Contexto:** D09 decía usuario y contraseña, y D49 y D50 lo concretaron con códigos por email. El maintainer quiere que entrar sea fácil (01/10/2026): registrarse con un email o con Google. Ninguna red social más por ahora: cada una es otra app que dar de alta y configurar (Meta para Instagram, por ejemplo), y se pueden añadir después.

**Decisión:**
- **Dos formas de entrar:** email y contraseña (con el código de D49 y la recuperación de D50) o **«Continuar con Google»**.
- **Desde una app nativa, por el navegador del sistema**, no dentro de la app: Google no deja iniciar sesión en vistas web incrustadas. La app abre el navegador en el servidor, que hace el OAuth con Google (en Symfony, `knpuniversity/oauth2-client-bundle`, con PKCE) y devuelve a la app un código de un solo uso, que la app cambia por el token de su dispositivo. En Android vuelve con un enlace a la app; en escritorio, igual o con una dirección local. Es el mismo camino en todas las plataformas.
- **Misma persona, misma cuenta:** si Google confirma un email que ya tiene cuenta, se enlaza con ella en lugar de crear otra. Una cuenta solo de Google no tiene contraseña que recuperar.
- **El servidor dice qué ofrece:** Google solo aparece si ese servidor lo tiene configurado. Quien monte el suyo sin Google sigue con email y contraseña.
- **Otro servidor:** la app entra por defecto en el oficial, y un enlace discreto, «Otro servidor», deja escribir la dirección de uno propio (como hace Bitwarden). Es lo que hace verdad que el servidor se pueda alojar.
- Iniciar sesión, con Google o sin él, sigue probando la clave del dispositivo (plan de la Fase 7) y queda en el registro de actividad (D66).
- **El email no se cambia** (02/10/2026): es lo que identifica la cuenta, también para enlazar con Google. Quien quiera pasar sus bibliotecas a otro email invita a ese email, lo hace administrador, y desde la cuenta nueva quita a la vieja (o la vieja se borra).

**Consecuencias:** hay que crear un proyecto en Google Cloud con su pantalla de consentimiento y sus credenciales, como para FCM (D75); se apunta en [development/servicios-externos.md](development/servicios-externos.md) para hacerlo paso a paso cuando llegue. La política de privacidad dirá qué se recibe de Google (email y nombre). Amplía D09 y D49.

## D78 · Desbloqueo con biometría y segundo factor, opcionales desde el principio — `aceptada`

**Contexto:** con D76 la cuenta vale tanto como el rol de su persona, así que protegerla bien importa. El maintainer quiere biometría y segundo factor desde el principio (01/10/2026), si son fáciles de usar y de configurar, y siempre opcionales.

**Decisión:**
- **Desbloquear la app con la huella o la cara**, en el móvil. Es local: protege que alguien con tu móvil abra la app, y funciona **también sin cuenta**. Plugin oficial `biometric` de Tauri (Android; en escritorio no hay). Se activa en Ajustes; apagado por defecto.
- **Segundo factor en la cuenta** con un código de una app de autenticación (TOTP: Google Authenticator, Aegis…) y **códigos de respaldo** para quien pierda el móvil. *Revisado el 02/10/2026:* primero decía `scheb/2fa-bundle`; se cambia por **`spomky-labs/otphp`** (la librería que ese bundle usa por debajo para TOTP) y un flujo propio. El bundle se apoya en el firewall y en la sesión, con un estado de «autenticado a medias»; la API no tiene sesión, solo un inicio de sesión que da un token, así que basta con un campo `totp_code` opcional en ese inicio de sesión. El mismo autor mantiene `web-auth/webauthn-lib`, útil para las llaves de acceso de más adelante. Cada usuario decide si lo activa. Como solo se inicia sesión una vez por dispositivo, molesta poco.
- **Propuesta, a revisar al construirlo:** el segundo factor se pide al entrar con email y contraseña; al entrar con Google lo protege el segundo factor de Google.
- **Quien aloja su servidor** decide si lo ofrece, con una variable de entorno; encendido por defecto.
- **Sin respaldo ni autenticador**, el superadministrador puede quitar el segundo factor por consola (D51), y queda en el registro de actividad (D66).
- **Llaves de acceso (passkeys), más adelante.** Tauri aún no tiene plugin oficial; si se construye la integración, la idea es publicarla como plugin libre de Tauri.

**Consecuencias:** entra en el hito 7.1 de la Fase 7. Pantallas nuevas para Design: activar el segundo factor (QR y códigos de respaldo), pedir el código al entrar, y el desbloqueo con biometría. Amplía D49, D50 y D76.

## D79 · Dominios: `.app` es lo que usa la app, `.com` es lo que se lee — `aceptada`

**Contexto:** el proyecto tiene kosmotheka.com y kosmotheka.app, y hasta ahora `.app` redirigía a `.com`. La Fase 7 necesita una dirección para la API, otra para las invitaciones y un dominio que verifique los enlaces de Android (App Links), que también hacen falta para volver de Google a la app (D77). Acordado con el maintainer el 01/10/2026.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| Un dominio principal (`.com`) y el otro redirige | Una sola web, una sola configuración | Mezcla en el mismo sitio lo que usa la app y lo que se lee |
| **Separados por función** | El enlace `kosmotheka.app/i/…` ya dice que abre la app; la web de `.com` puede alojarse en cualquier sitio y no cae con el VPS; en `.app` no hay contenido que mantener | Algunas cosas se configuran dos veces: los dos dominios en Google (OAuth, Firebase y Search Console), dos certificados y dos DNS |

**Decisión:** separados, con la regla «**`.app` es lo que usa la app, `.com` es lo que se lee**».

| Dirección | Qué es |
|---|---|
| `api.kosmotheka.app` | El servidor de la app |
| `kosmotheka.app/i/…` | Las invitaciones (D43) |
| `kosmotheka.app/.well-known/assetlinks.json` | El fichero que verifica los enlaces de Android |
| `kosmotheka.app` | Una página de instalación sencilla que remite al `.com` |
| `kosmotheka.com` | Landing, documentación, descargas, políticas y contacto (Fase 8) |

- La redirección de `.app` a `.com` desaparece. La política de privacidad vive solo en `.com`, y la página de instalación y la pantalla de consentimiento de Google enlazan allí.
- Android comprueba los enlaces verificados en el dominio exacto y no sigue redirecciones: todos los enlaces que genere la app son de `kosmotheka.app`.

**Quien aloje su servidor lo configura a su manera:** la dirección de la API es la que escribe en «Otro servidor» (D77), y la de las invitaciones, una variable de entorno del servidor; puede usar uno o dos dominios. **Límite:** los enlaces verificados de Android se declaran al compilar la app oficial, así que solo valen para `kosmotheka.app`. En un servidor propio, la página de la invitación abre la app con el esquema propio (`kosmotheka://`), que funciona con cualquier servidor, o deja copiar la invitación para pegarla en la app; y volver de Google usa también ese esquema, con el cliente OAuth que configure quien lo aloja.

**Consecuencias:** se apunta en [development/servicios-externos.md](development/servicios-externos.md). Concreta D22 (la web pública va en `.com`) y D74 (la página de descargas, en `.com`).

## D80 · Correo: `info@kosmotheka.com` reenviado a Gmail, envíos con Brevo — `aceptada`

> Revisada el 01/10/2026, el mismo día: la primera versión elegía el plan gratuito de Zoho Mail, pero Zoho no lo ofrece en nuestra región.

**Contexto:** hace falta una dirección del proyecto para dar de alta los servicios externos (Google Cloud, Firebase, Play…) y para que nos escriban desde la web y Google Play, y hay que poder responder. Además, el servidor envía los códigos de registro y de recuperación (D49, D50). Requisitos del maintainer (01/10/2026): sin coste, responder con la dirección del dominio y el DNS se queda en el proveedor del VPS.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| Google Workspace | Lo más cómodo | Unos 5 € al mes |
| Zoho Mail gratis | Buzón real con app | No se ofrece en nuestra región |
| Zoho Mail Lite | Buzón real con IMAP, un solo servicio | Unos 12 € al año |
| Reenvío a la bandeja Conversations de Brevo | Gratis, con app | Es una bandeja de atención al cliente, no un buzón; si Brevo suspende la cuenta, se pierden a la vez el buzón y los envíos |
| Buzón de pago pequeño (Purelymail, Migadu) | IMAP estándar | Cuesta dinero; son servicios pequeños o limitan los envíos |
| Servidor propio (Stalwart, docker-mailserver) | El buzón es nuestro | el proveedor del VPS bloquea el puerto 25 de salida, así que hace falta un relay igualmente; copias y mantenimiento a nuestro cargo |
| Solo una cuenta de Gmail | Gratis, buen buzón | Se responde y se dan de alta los servicios con una dirección de Gmail; cambiar después obliga a tocar cada cuenta |
| **Dominio por fuera, Gmail por dentro** | Gratis; todo sale con el dominio; cambiar el buzón es cambiar los MX | Tres piezas; responder como `info@` desde Gmail deja de funcionar en enero de 2027 |

**Decisión:** el dominio por fuera y Gmail por dentro.

- **`info@kosmotheka.com` es la dirección del proyecto** y vive en `kosmotheka.com`, el dominio «que se lee» (D79). **ImprovMX** (gratis) la reenvía al buzón de Gmail del proyecto.
- **Todos los servicios se dan de alta con `info@kosmotheka.com`**, nunca con el Gmail, también la cuenta de Google para Google Cloud y Firebase (se puede crear una cuenta de Google con un email que no es de Gmail). Así cambiar el buzón solo toca el DNS.
- **Se responde como `info@`** desde Gmail con «Enviar como», a través del SMTP de Brevo. Google lo retira para direcciones ajenas en **enero de 2027**: entonces el buzón pasa a Zoho Mail Lite o a lo que convenga, cambiando solo los MX, o se responde desde otra app de correo con esa identidad.
- **El servidor envía con Brevo** a través de `symfony/mailer` (`MAILER_DSN`), desde `no-reply@kosmotheka.com`. Si Brevo falla o suspende la cuenta, se cambia el DSN a otro proveedor (Resend, Mailjet, Amazon SES); recibir no se ve afectado.
- **No se envía con el SMTP de Gmail** desde el servidor: pide contraseñas de aplicación y saldría desde una dirección de Gmail.

**Consecuencias:** el DNS de `kosmotheka.com` lleva los MX de ImprovMX, un único SPF con ImprovMX y Brevo, el DKIM de Brevo y DMARC. Los pasos están en [development/correo.md](development/correo.md), y la tabla de [development/servicios-externos.md](development/servicios-externos.md) queda actualizada. **Pendiente para enero de 2027:** decidir el buzón definitivo. Quien aloje su servidor usa el proveedor que quiera con `MAILER_DSN` (D18). Resuelve lo que D49 y D50 dejaban pendiente.

## D81 · Dar un dispositivo a otra persona: la identidad es de la instalación y se puede empezar de cero — `aceptada`

**Contexto:** una costura que las fases 4 a 6 dejaron sin validar, vista al revisar las cuentas del hito 7.1 (02/10/2026). El caso típico: me compro un móvil nuevo y el viejo se lo doy a mi hijo, o a mi pareja, con la app instalada.

- La identidad de un dispositivo es **de la instalación de la app**, no del móvil: la primera vez que se abre, la app genera su id (el de su `hlc`) y su clave Ed25519 (D56). Restaurar el móvil de fábrica o reinstalar la app ya crea un dispositivo nuevo; ese caso no da problemas.
- Con la app tal cual, en cambio, quien recibe el móvil **sigue siendo yo** para las bibliotecas: cada biblioteca sabe de quién es ese dispositivo y le da el rol de esa persona (D73), y sus operaciones llevan mi firma. Iniciar sesión con otra cuenta no lo arregla: por la red local los demás dispositivos siguen viendo mi firma. Además, en el móvil siguen todas mis bibliotecas.
- El servidor (hito 7.1) liga para siempre el id de un dispositivo a la cuenta que lo dio de alta, y solo con su clave. Es correcto y se mantiene: si otra cuenta pudiera reclamar ese id, podría firmar como yo. Lo que faltaba es que la app pueda **cambiar de identidad**.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| Dejar que otra cuenta reclame el id en el servidor | Nada que hacer en la app | La clave sigue siendo la misma: firmaría como yo, por la red local y en la nube |
| Cambiar de identidad en cada «Cerrar sesión» | Una sola acción | Quien cierra sesión y vuelve a entrar pierde su vínculo con su persona en las bibliotecas que no están en la nube |
| **Dos acciones distintas, y quitar el dispositivo desde otro** | Cada caso hace lo que se espera; sirve también sin cuenta | Una opción más en Ajustes que hay que explicar bien |

**Decisión** (maintainer, 02/10/2026):
- **«Cerrar sesión»** es para la misma persona, que volverá a entrar: no borra nada ni cambia la identidad. Al volver a iniciar sesión con la misma cuenta sigue siendo el mismo dispositivo.
- **«Dejar este dispositivo a otra persona»**, en Ajustes, deja la app como recién instalada:
  1. ofrece hacer una copia antes (como D65) y avisa de lo que se borra;
  2. si hay cuenta, cierra la sesión y avisa al servidor de que el dispositivo se ha quitado;
  3. borra **todas** las bibliotecas del dispositivo, con su registro;
  4. genera un id y una clave nuevos.
  Quien lo recibe entra con su cuenta, o sin cuenta, como un dispositivo nuevo: recibe las bibliotecas compartidas con su propio rol cuando lo vinculan o cuando la nube lo hace (D76). **Funciona también sin cuenta** (D57), que es el caso que ya existe hoy con la red local.
- **Si el móvil ya se dio sin hacerlo**, se quita desde otro dispositivo de la misma cuenta, en la lista de dispositivos. El servidor distingue dos estados:
  - **sesión cerrada:** al volver a iniciar sesión con la misma cuenta sigue siendo el mismo dispositivo;
  - **quitado:** la próxima vez que esa instalación hable con el servidor recibe el error `device_removed`, y la app hace lo mismo que «Dejar este dispositivo a otra persona» (sin la copia, porque ya no es de quien lo quitó) y crea su identidad nueva. **Con ella puede entrar cualquier cuenta, o usarse sin cuenta**: lo único que no vuelve a entrar es la identidad vieja, que firmaría como su anterior dueño.
- **Quitar uno por error** (aclarado con el maintainer el 02/10/2026): si se quita mi portátil sin querer, al conectarse borraría sus bibliotecas; las de la nube vuelven al entrar otra vez, pero las que solo estaban en él se perderían. Por eso:
  - el aviso al quitarlo dice qué va a pasar, y si el dispositivo tiene bibliotecas que no están en la nube, que se perderán (lo sabe la app por las bibliotecas que ese dispositivo comparte con la nube);
  - **se puede deshacer mientras ese dispositivo no se haya conectado**: hasta entonces no ha borrado nada, y al deshacerlo sigue como estaba, con su sesión. En cuanto se conecta y recibe `device_removed`, ya no se puede.
  - No se ofrece hacer una copia en el dispositivo quitado: la haría quien lo tiene ahora, con los datos de otra persona.
- **En las bibliotecas**, quitar un dispositivo es además **retirarle el acceso** (D71) en cada biblioteca compartida que tenga, para que tampoco reciba nada por la red local ni pueda cambiar nada. Sin cuenta, se hace como hasta ahora: retirando el acceso a ese dispositivo desde la biblioteca.

**Consecuencias:**
- **Servidor (hito 7.1):** el dispositivo tiene el estado «quitado», distinto de «sesión cerrada»; `DELETE /v1/account/devices/{id}` lo quita, un token de un dispositivo quitado responde `device_removed`, y el servidor apunta cuándo se lo dijo por primera vez: hasta entonces, `POST /v1/account/devices/{id}/restore` lo deshace. Ver [architecture/accounts.md](architecture/accounts.md).
- **App:** la acción de Ajustes, el borrado completo y la identidad nueva (un id y una clave nuevos en `local_state`) van con las pantallas de la cuenta (hito 7.1); la parte sin cuenta no depende de la nube y puede adelantarse. La retirada en las bibliotecas al quitar un dispositivo, con el hito 7.5. Pantallas nuevas para Design y textos en [design/errors.md](design/errors.md).
- **Límite, como en D71:** lo que quien recibe el móvil haya visto o sacado antes de que se quite no se puede deshacer. La biometría (D78) protege el caso de un móvil que se da o se pierde sin hacer nada.
- **Auditoría:** el caso entra en las costuras de compartir (`flows-sharing`).
- Amplía D56, D57, D71, D73 y D76.

## D82 · Portadas propias (foto o fichero) y cómo viajan las imágenes entre dispositivos — `abierta`

**Contexto** (maintainer, 03/10/2026, probando la versión de Google Play):
- Quiere **sacar una foto a la portada** con el móvil, o elegir una imagen del dispositivo. Hoy está desactivado: D27 solo guarda portadas **descargadas** de los servidores conocidos (Open Library y Google Books), y dejó pendiente la portada propia.
- La cámara no es el problema: el WebView de Android de Tauri ya abre la cámara desde un `<input type="file" accept="image/*" capture>`.
- **El problema es el transporte.** La portada es un fichero `covers/<sha256>.<ext>` y la operación solo lleva su nombre (D27). Hoy la sincronización (red local y nube) mueve operaciones, no ficheros: un dispositivo que recibe la edición conoce el nombre de la portada, pero no tiene el fichero. Una portada descargada se podría volver a bajar de su URL; una foto propia no está en ninguna URL.
- De paso: algunas portadas encontradas se ven **muy pequeñas** (la miniatura de Google Books), y al pulsar «Lo he comprado» en «¿Lo tengo?» la portada encontrada no pasaba al formulario (arreglado el 03/10/2026).

**Preguntas a decidir:**

1. **Cómo viaja una imagen.**

| Opción | A favor | En contra |
|---|---|---|
| Dentro de la operación (base64) | Nada nuevo en el protocolo | Operaciones enormes que se guardan para siempre en el historial y se reenvían en cada sincronización; rompe el límite de tamaño de las operaciones |
| Volver a descargarla de su URL en cada dispositivo | Simple | No sirve para fotos propias; depende de que el proveedor siga ahí |
| **Ficheros aparte, pedidos por su hash después de sincronizar** | Las operaciones siguen pequeñas; el hash ya identifica el contenido (D27) y sirve de comprobación; dos dispositivos con la misma portada no la piden | Un canal nuevo en la red local y almacenamiento de ficheros en el servidor |

   Propuesta: **ficheros aparte por hash**. Tras sincronizar operaciones, cada lado pide las portadas que le falten.
   - **Red local:** un endpoint del servidor de la red local que entrega una portada por su hash, solo a dispositivos emparejados y solo si pertenece a una biblioteca que ese dispositivo puede recibir (D71).
   - **Nube:** el servidor guarda las portadas por hash, con un límite por biblioteca. Las sube quien tiene la portada; las baja quien la necesita.
   - **Al recibir:** se recalcula el hash y se rechaza si no coincide, igual que al restaurar una copia (D30); solo imágenes, con tamaño máximo.

2. **Tamaño y privacidad de las fotos.** Propuesta: en el dispositivo, antes de guardarla, **reducir** a unos 1000 px por el lado largo, **recomprimir** (unos 100 a 200 KB) y **quitar los metadatos EXIF**, que en una foto del móvil pueden llevar la ubicación de casa.

3. **Quién la cambia.** Cambiar la portada es editar la edición: quien puede editar lo hace; quien solo consulta lo pide (D55).

4. **Portadas grandes de los proveedores:** pedir a Google Books una imagen mayor que la miniatura cuando la hay.

5. **Limpieza:** las portadas que ya no usa ningún libro se quedan en disco (pendiente desde D27); con ficheros que viajan conviene limpiarlas.

**Consecuencias:**
- La spec no cambia: el campo `cover` ya existe. Cambian el protocolo de la red local (un endpoint nuevo) y el servidor (almacenamiento de ficheros y su API), y la copia de seguridad sigue igual (ya lleva las portadas).
- **Cuándo:** propuesta de hacerlo en su propio hito, **después de la 7.2**, para no mezclarlo con las cuentas. Tiene que estar antes de que la nube sincronice catálogos, para que no se diseñe dos veces.
- Amplía D27.

## D83 · Google Books sin clave no basta: clave propia, servidor solo con cuenta y la BNE — `aceptada` (la BNE, por investigar)

**Contexto** (03/10/2026, probando la versión de Google Play). *Inferno* de Dan Brown (Planeta, 9788408176039) no rellenó nada: Open Library e Inventaire no lo tienen, y Google Books respondió **429, «cuota diaria agotada»**. Es lo que ya vio D26: sin clave, todas las peticiones anónimas del mundo comparten una cuota que se agota a menudo. Para el público de ahora (libros en español), la fuente que mejor los cubre falla justo para quien no pone clave, que es casi todo el mundo. El aviso de la app se fue tan rápido que no dio tiempo a leerlo.

| Opción | A favor | En contra |
|---|---|---|
| Una clave de Kosmotheka dentro de la app | Funciona para todos sin hacer nada | En escritorio no se puede atar a la app: cualquiera la saca del binario y gasta la cuota de todos |
| Consultar siempre a través de nuestro servidor | La clave no sale del servidor; caché por ISBN | Usar el servidor sin cuenta contradice que sin cuenta no se hace nada en el servidor (D18, D57) |
| **Clave propia siempre; el servidor solo para quien tiene cuenta, avisando; y más fuentes abiertas** | Respeta el modo sin cuenta; quien tiene cuenta lo tiene resuelto | Sin cuenta ni clave, Google sigue fallando a menudo: hacen falta otras fuentes |

**Decisión** (maintainer, 03/10/2026):
- **No hay clave de Kosmotheka en la app.**
- **Sin cuenta, nada pasa por el servidor**, tampoco las búsquedas.
- **Con cuenta**, las búsquedas de Google Books pueden ir por el servidor, que pone su clave y guarda una caché por código. **Se avisa** de ello en Proveedores (qué se envía: el código o el título buscado, nada más) y se puede apagar.
- **Clave propia, siempre**: en Ajustes › Proveedores, como ahora, y tiene preferencia. Con un **botón de ayuda** que explica cómo conseguirla y enlaza a la página de Google Cloud donde se crea.
- **Biblioteca Nacional de España (BNE)**, si se consigue hacer funcionar: abierta, sin clave y muy buena en lo publicado en España, que es el público de ahora. Hay que investigar su API (datos.bne.es o el catálogo); la primera prueba no respondió como se esperaba.
- **Más adelante, otras bibliotecas por idioma** como fuentes adicionales: de momento solo para inglés.

**Que la lista no crezca sin control.** Una fuente nueva entra solo si cumple todo esto:
1. **cubre algo que las demás no**, medido con una lista fija de códigos de prueba por idioma y tipo (como `tests/fixtures/`): qué porcentaje encuentra cada fuente y cuánto añade sobre las que ya hay;
2. es **abierta o gratuita** y sus condiciones permiten usarla desde una app;
3. responde en un tiempo razonable y tiene una API estable;
4. alguien la mantiene: cada fuente es código, pruebas y respuestas grabadas que se rompen cuando cambian.

   Las fuentes se **eligen por idioma**: por defecto, las del idioma de la app y del dispositivo, y en Proveedores se encienden o apagan las demás. Así la lista puede ser larga sin que cada búsqueda consulte todas. Cuando el servidor busque para quien tiene cuenta, puede añadir fuentes sin actualizar la app.

**Consecuencias:**
- **Aviso de fallo visible:** el aviso de un proveedor que falla no puede desaparecer antes de leerlo. Debe quedarse junto al resultado («Google Books no ha respondido: cuota agotada. Pon tu clave en Proveedores»), con enlace a Proveedores.
- **Servidor:** un endpoint de búsqueda para cuentas, con caché y límite por cuenta; entra con la nube (fase 7), después de la 7.2.
- **BNE:** investigación y proveedor en su propio paso, con respuestas grabadas como los demás (D18).
- Amplía D26.

## D84 · Acceso a la nube: id propio de la nube y tabla de acceso; el servidor no se fía de ningún id del cliente — `aceptada`

> Aceptada por el maintainer el 03/10/2026, con dos condiciones para construir 7.2: fixtures que simulen todos los casos, normales y costuras, para ver la base de datos; y el código del servidor, revisado y refactorizado por él.

**Contexto:** al preparar el hito 7.2 (03/10/2026) salió que el plan copiaba a la nube la lógica de la red local: «la nube es un dispositivo más, con el mismo protocolo» (D57). En la red local la puerta la guarda una persona: nadie entra sin ver el QR, y el código puede fiarse de sus compañeros («una biblioteca que me ofrecen y no tengo, la acepto»). A la nube le habla cualquiera con una cuenta. Copiada tal cual, esa lógica convierte el id de una biblioteca en su llave, y «Mi biblioteca» tiene el mismo id en todas las instalaciones (D46): dos desconocidos que la subieran quedarían fundidos. La auditoría del mismo día ([2026-10-03-2](audits/2026-10-03-2.md), SEG-010 a SEG-013 y DAT-001) lo confirmó y encontró además dos fallos de la red local.

Un ejemplo para medir cualquier diseño: **Jhonny, en Nueva York, no conoce de nada a una familia de España, pero es hostil y conoce sus ids** (de biblioteca, de operaciones y de dispositivos). No debe poder recibir nada de ella, ni escribir en ella, ni estorbarla.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| La nube decide con el id de la biblioteca (plan anterior) | Mismo código que la red local | El id es la llave: el fijo lo conoce todo el mundo y los demás viajan en vectores y copias |
| Quitar el id fijo duplicando «Mi biblioteca» al actualizar | Ninguna instalación compartiría id | Cada dispositivo de una familia duplicaría por su cuenta: bibliotecas repetidas y sin historial. El id local seguiría siendo la llave para todos los demás |
| **Dos niveles: datos iguales, puerta tradicional** | Un id local repetido deja de importar en la nube; el acceso es el de cualquier servicio con cuentas | El servidor tiene su propia clave para las bibliotecas; hay que vigilar cada sitio donde el id local pueda colarse |

**Decisión:** dos niveles.
- **Datos, igual que hoy:** las mismas operaciones firmadas, las mismas reglas de fusión y los mismos roles (D32, D56, D73). Una operación que llega por la red local y por la nube es la misma y se aplica una vez.
- **Puerta, como una app tradicional:**
  1. **Id propio de la nube.** Al subir una biblioteca, el servidor crea un `cloud_library_id` aleatorio. Todo en PostgreSQL cuelga de él. El id local no da acceso a nada.
  2. **Tabla de acceso** (`membership`: cuenta, biblioteca de la nube y persona, D76). La nube entrega una biblioteca solo a dispositivos, no retirados ni quitados, de cuentas que estén en esa tabla. Nadie entra en la tabla sin consentimiento: una persona sin cuenta (un niño) no aparece en ella, sigue por la red local y lo suyo llega a la nube como parte de la biblioteca, por decisión de quien administra (D76).
  3. **Una operación firmada une las dos identidades.** Al subir, un dispositivo de una persona administradora registra en la biblioteca «esta biblioteca es `C` en la nube de `<servidor>`». Viaja a todos los dispositivos. Un dispositivo solo conecta su biblioteca local con `C` si **su propio registro** tiene esa operación; si la registra quien no puede compartir, los demás la juzgan como cualquier operación y queda en petición (D73).
  4. **El servidor no se fía de ningún id del cliente.** El id local de la biblioteca, los ids de operación y los ids de dispositivo que llegan en una petición se comparan, nunca se usan para buscar ni para decidir.
- **Lo que no cambia:** la red local (D34, D71), el modo sin cuenta (D57) y el id fijo de las instalaciones que ya lo tienen (D46). Las instalaciones nuevas pueden crear su primera biblioteca con id aleatorio, como mejora, pero ya no es una medida de seguridad.

**Costuras de la nueva clave, cada una con su test del caso hostil** (lista completa en [plans/fase-7.md](plans/fase-7.md), «Batería de acceso»):
1. Ids de operación únicos **dentro de cada biblioteca de la nube** (`(cloud_library_id, operation_id)`), nunca en toda la nube: si no, Jhonny sube antes operaciones con tus ids y las tuyas se descartan como «ya conocidas». También lo necesitan dos administradores que suben la misma biblioteca a la vez.
2. Las peticiones de sincronización dicen `C`, no el id local; el servidor no traduce uno en otro.
3. Cada `C` queda atada al id local de su operación de enlace; el servidor rechaza en `C` operaciones de otra biblioteca.
4. Las claves de firma se buscan dentro de cada `C`, nunca en una tabla global por id de dispositivo.
5. Avisos al momento (7.3), invitaciones (7.6) y registro de actividad (7.7) cuelgan de `C`: un tema de Mercure con el id local dejaría a Jhonny saber cuándo cambia algo.
6. Ids de dispositivo de las cuentas (7.1): el primero que usaba un id se lo quedaba, y quien conociera el id de tu móvil podía ocuparlo con su clave y dejarte fuera (SEG-016). **Arreglado el 03/10/2026:** para el servidor, un dispositivo es su id junto con la clave que prueba al iniciar sesión.
7. La nube rechaza solo por lo que dice la propia operación (estructura, versión, firma), nunca por el orden de llegada (DAT-001); lo no permitido pasa a petición (D73).
8. Dos administradores que suben sin verse crean `C1` y `C2` para la misma biblioteca: gana el enlace de HLC más reciente (la regla normal campo a campo, D32; ajustado el 03/10/2026, antes decía «el más antiguo»: lo que importa es que sea determinista), los dispositivos del otro se pasan a esa y la sobrante se borra. No se pierde nada: son las mismas operaciones.

**Resuelto al estudiarla (03/10/2026), cada uno con su test visto en rojo y después en verde:**
- **Antedatar (SEG-014, confirmado el 03/10/2026 por `a_demoted_editor_cannot_backdate_a_change`; decidido el mismo día: opción B, «marca en la bajada»):** el rol se juzga en el HLC de la operación, y el HLC lo pone quien firma. A quien bajaron de editor a lector podía, con una app modificada, firmar un cambio con fecha anterior a la bajada, y se aplicaba en todos los dispositivos. Una operación hecha sin conexión antes de enterarse y una antedatada son indistinguibles sin una hora de confianza, así que:
  - **Cada operación que quita poder a una persona lleva una marca por cada dispositivo suyo:** bajar de rol, quitar «Puede compartir», cambiar de quién es un dispositivo o dejarlo sin persona, y retirar el acceso (D71). La marca es un campo nuevo del dispositivo, `trusted_until`: el HLC de la última operación de ese dispositivo que tiene quien la registra.
  - **Ventana vigilada:** una operación de ese dispositivo con HLC posterior a la marca y anterior a la operación que la puso **pasa a petición**, llegue cuando llegue. Lo anterior a la marca ya lo conocía quien administra (las operaciones de un autor llegan siempre en orden y completas), así que no puede ser nuevo de forma honrada.
  - **Cuenta cada marca, no solo la última:** el campo del dispositivo guarda la vigente, pero las ventanas salen del registro, para que una segunda bajada no reabra la ventana de la primera.
  - **Solo cuentan las marcas permitidas:** las pone un administrador (como todo cambio en el dispositivo de otro). Al recalcular una biblioteca se parte de todas las marcas del registro, se descartan las que hayan quedado en petición y se repite hasta que no cambie nada; termina siempre y da lo mismo en todos los dispositivos.
  - **Coste aceptado:** una edición honrada hecha sin conexión justo antes de enterarse de la bajada queda como petición, y quien administra la acepta. Nadie pierde nada.
- **Retirada a través de un tercero (SEG-015, arreglado el 03/10/2026):** retirar el acceso solo cortaba el intercambio directo; lo que el dispositivo retirado firmaba después llegaba a todos a través de otro que aún no lo sabía, y se aplicaba. Ahora una retirada en vigor es una marca abierta: lo que ese dispositivo firme después pasa a petición, con roles y sin ellos, hasta que se le devuelva el acceso.

**Operaciones sin firma (decidido el 03/10/2026):** primero se decidió que la nube las rechazara y que la app no subiera una biblioteca con historia sin firmar; el mismo día el maintainer amplió la regla a todas partes: toda operación va firmada y una sin firma se rechaza en cualquier dispositivo y en la nube (D56, D73). Así nube y dispositivos aplican el mismo criterio y no hay huecos. La app sigue sin subir una biblioteca con historia sin firmar y propone «Duplicar como nueva» (D58). Los datos de antes de las firmas son de prueba y pueden perderse.

**Pendiente hasta 7.4:**
- **Un lector con una app modificada** puede subir una copia a su propia cuenta. No obtiene nada que no tuviera (D73), y nadie de su familia se conecta a esa copia (punto 3), pero hasta 7.4 el servidor no sabe de roles para impedirlo (SEG-013).

**Consecuencias:**
- **D57** se matiza: la nube es un dispositivo más **en los datos**, no en la puerta.
- **D41:** la clave del catálogo en PostgreSQL pasa a `(cloud_library_id, id)`. No hay datos que migrar: el servidor aún no tiene catálogo.
- **D76:** se mantiene, apoyada en la operación de enlace del punto 3 y en la prueba de clave al iniciar sesión.
- **D46:** no cambia.
- **Plan de la Fase 7:** 7.2 se rehace con este modelo; antes van los arreglos de la red local (SEG-010, SEG-011) y la batería de acceso.
- **Pruebas:** toda regla de quién recibe una biblioteca tiene su test del caso hostil, en Rust, en PHP o en los dos ([AGENTS.md](../AGENTS.md)). La auditoría es la segunda línea, no la primera; el tema `library-access` vuelve tras 7.2, 7.5 y 7.6, y `threat-model` se rehace al aceptar esta decisión.

## D85 · Adiós al id fijo de «Mi biblioteca»: toda biblioteca nace con id aleatorio — `aceptada`

> Aceptada por el maintainer el 03/10/2026. Reemplaza el punto «Migración con continuidad» de D46.

**Contexto:** D46 dio a la primera biblioteca un id fijo igual en todas las instalaciones (`00000000-0000-7000-8000-000000000001`), para que las que ya se sincronizaban antes de existir las bibliotecas se fundieran sin duplicados. Ese id ha estado detrás de todo lo que salió el 03/10/2026: la fusión entre desconocidos en la nube (resuelta por la puerta de D84), SEG-010, y, ya con D84 construida, que el móvil nuevo de alguien no pudiera bajar su propia «Mi biblioteca», porque ese móvil nace con otra vacía con el mismo id y D84, con razón, no las mezcla. D84 es la seguridad y se queda; el id fijo era un problema de uso para la gente honrada. La opción de quitarlo se descartó al preparar D84 por no perder datos de las familias; el mismo día el maintainer dijo que los datos de ahora son de prueba y pueden perderse, y con eso desaparece el motivo.

**Decisión:**
- **Ninguna biblioteca tiene un id fijo.** La primera se crea al primer arranque con id aleatorio y su operación `library.create` firmada, como cualquier otra, así que viaja igual que las demás. La biblioteca activa por defecto es la primera que haya.
- **Instalaciones existentes, una sola vez al actualizar:** si su biblioteca con el id fijo está vacía, se quita y se crea la primera como en una instalación nueva; si tiene datos, se duplica como nueva (D58: los datos se conservan con operaciones firmadas, el historial no) y la vieja se quita de ese dispositivo. Los dispositivos que la compartían por la red local la vuelven a compartir.
- **Se quedan las defensas generales**, que no dependían del id fijo: nunca fusionar una biblioteca que tu registro no enlazó (D84), SEG-010 y la puerta de la nube. Protegen igual contra un id repetido a propósito.

**Consecuencias:**
- D46: su punto «Migración con continuidad» queda reemplazado; el resto sigue.
- D84: deja de necesitar el caso especial del id fijo; sus costuras y tests siguen valiendo para cualquier id repetido.
- Los tests que simulaban «una biblioteca compartida» usando el mismo id fijo en dos dispositivos crean la biblioteca en uno y la reciben en el otro, como en la realidad.

## D86 · Copias de seguridad cifradas con contraseña, activado por defecto — `aceptada`

> Aceptada por el maintainer el 03/10/2026, al preguntar qué pasa si a alguien le roban su copia.

**Contexto:** una copia (D30, D58) es un zip sin cifrar con el catálogo en JSON, las portadas y el historial. No lleva claves, tokens ni secretos (D58), así que quien la encuentre no puede firmar, sincronizar ni entrar en ninguna cuenta, y con D84 y SEG-016 los ids que contiene no abren ninguna puerta. Pero sí lee datos personales: nombres (también de niños), qué hay en casa y dónde, qué lee cada uno, valoraciones y préstamos. Las copias suelen acabar en servicios como Google Drive, que se pueden hackear.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| Siempre cifrada | Ninguna copia legible fuera de la app | Si se olvida la contraseña, la copia no sirve |
| **Opcional, activada por defecto** | Cifrada salvo que se quite a propósito, sabiendo lo que implica | Puede haber copias legibles, por decisión de quien la hace |
| Opcional, desactivada | Como hasta ahora | La mayoría de copias quedarían legibles |

**Decisión:** opcional, activada por defecto.
- Al crear una copia, «Proteger con contraseña» viene marcado; quitarlo explica que la copia quedará legible para quien la encuentre.
- **Criptografía estándar, pocas dependencias** (cambiado el 03/10/2026, revisado con el maintainer): `scrypt` para sacar la clave de la contraseña y `chacha20poly1305` para cifrar, los dos de RustCrypto. Solo el «sobre» del fichero es nuestro (cabecera con versión, parámetros de scrypt, sal, nonce y datos cifrados), **documentado paso a paso** en `docs/architecture/data-model.md` para que la comunidad lo pueda auditar y cualquiera pueda descifrar su copia con unas líneas en otro lenguaje.
  - Primero se construyó con el formato `age` (crate `age`), para poder abrir la copia sin la app con la herramienta `age`. Trajo unas 70 dependencias nuevas a la app (traducciones propias, criptografía post-cuántica que no se usa): demasiado peso y superficie de ataque para lo que aporta. **Sustituido el 04/10/2026:** el árbol de dependencias de la app bajó de 601 a 506 crates; el formato está en `docs/architecture/data-model.md`, con un script de Python que descifra una copia sin la app (comprobado contra una copia real). Las copias `age` de esos días de desarrollo no se abren: la app lo dice (`old_encrypted_backup`).
- Al restaurar, la app detecta si la copia está cifrada y pide la contraseña; con una contraseña equivocada no restaura nada.
- La contraseña no se guarda en ningún sitio: si se pierde, la copia no se puede abrir, y la pantalla lo dice antes de crearla.
- **Un test garantiza** que ninguna copia, cifrada o no, contiene la clave de firma, el token de la cuenta, los secretos de emparejamiento ni la identidad TLS.

**Consecuencias:** cambia el formato de las copias (una copia cifrada es el sobre de `data-model.md` con el zip de siempre dentro); las copias antiguas sin cifrar se siguen restaurando. Exportar a CSV sigue siendo legible: es para llevarse los datos a otra herramienta, y la pantalla lo avisa.

## D87 · Cada operación encadena la anterior de su autor — `aceptada`

> Aceptada por el maintainer el 04/10/2026 («si eso ofrece seguridad y no rompe nada, adelante»), al estudiar SEG-018 en la auditoría 2026-10-03-3.

**Contexto:** la sincronización por vectores guarda de cada autor solo su operación más alta y da por hecho que tiene todas las anteriores. En la auditoría 2026-10-03-3 aparecieron cuatro caminos que dejaban un hueco por debajo sin que nadie lo viera (DAT-001 a DAT-004), y SEG-018: quien pierde un rol puede firmar con una app modificada una operación nueva con fecha anterior a la marca de su valla (D84), y se juzga con el rol antiguo. La primera regla que se propuso («una operación por debajo de lo que ya tengo de su autor es falsa») no sirve: cada dispositivo la juzgaría según lo que tuviera al recibirla, y las copias divergirían.

**Opciones:**

| Opción | A favor | En contra |
|---|---|---|
| A · Aceptar SEG-018 como riesgo | Coste cero | Los huecos siguen siendo invisibles |
| B · Encadenar las operaciones de cada autor | Cierra SEG-018 con una regla que depende solo del registro; cada dispositivo sabe cuándo le falta una operación | Cambia el formato de las operaciones en `spec/`, Rust y PHP |

**Decisión: B.**
- Cada operación lleva `previous`, firmado con el resto: el id de la operación anterior del mismo dispositivo en la misma biblioteca, o `null` si es la primera.
- **Al recibir**, una operación cuya anterior no se conoce espera, y con ella las siguientes de su autor; no se salta nunca. La nube hace lo mismo al recibir un `push`.
- **Las vallas apuntan a una operación, no a una fecha.** Al quitar un rol o retirar el acceso, la marca (`trusted_through`) es el id de la última operación del dispositivo que tenía el administrador. Mientras dure la valla, solo cuentan como suyas de antes la operación marcada y sus anteriores en la cadena; cualquier otra fechada dentro de la valla es una petición. Una retirada es una valla sin final (SEG-015). La regla depende solo del contenido del registro, así que todos los dispositivos llegan al mismo resultado al recalcular.
- **Sin excepciones para historia antigua:** una operación sin `previous` no es válida. Las bibliotecas de prueba anteriores se recuperan con «Duplicar como nueva» (D58), como la historia sin firmar.

**Consecuencias:**
- Sube la versión del formato (`schemaVersion`); D31 sigue igual.
- `trusted_until` (una fecha) pasa a `trusted_through` (un id de operación) en `device`, con su migración, `schema.rs`, `spec/` y `CatalogEntity.php`.
- **Límite que queda:** un editor con una app modificada puede bifurcar su propia cadena (dos operaciones con la misma anterior) y que solo la reciban los dispositivos que iban por detrás. La cadena lo hace detectable, pero no se resuelve aquí; queda en el modelo de amenazas como riesgo aceptado (solo un miembro con permiso de edición, y solo con lo que ya podía hacer).
- Una entidad que se quite del esquema en el futuro no puede dejar operaciones sin enviar en medio de una cadena (como pasó con `wish`, D61): habrá que convertirlas, no saltarlas.
- Glosario: «operación anterior del autor» = `previous`.

## D88 · Emparejar una vez: los dispositivos de una biblioteca se reconocen solos en la red local — `abierta`

**Contexto:** hoy la confianza en la red local va de dos en dos: cada pareja de dispositivos se empareja con un QR (D13), uno hace de servidor y otro de cliente. En una familia con portátil, dos móviles con nube y el de un hijo sin cuenta, eso obliga a emparejar varias veces o deja una cadena que se corta: si el móvil que hace de puente se va de viaje, los demás dejan de hablarse aunque estén en la misma casa. El maintainer (04/10/2026): emparejar varias veces es incómodo para el usuario y hay que mejorarlo; **se hará**, aunque no ahora.

**Idea de partida:** cada dispositivo ya tiene su clave pública firmada dentro de cada biblioteca (D56, D73). Dos dispositivos que comparten una biblioteca podrían encontrarse solos en la red de casa y sincronizar esa biblioteca sin haberse emparejado entre ellos: se empareja una vez con cualquiera de la biblioteca y después se habla con quien esté.

**Por decidir (con tests hostiles antes, como pide la batería de acceso):**
- Quién deja entrar: ¿basta con estar en el registro de la biblioteca, o hace falta que un administrador lo haya aceptado (D73)?
- Cómo se autentican dos dispositivos sin secreto compartido: TLS con la clave de cada uno, comprobada contra la que la biblioteca tiene registrada.
- Cómo se encuentran: descubrimiento en la red local (mDNS) o direcciones que viajan con la biblioteca; qué se anuncia en una red que no es de casa (el nombre del dispositivo, nunca el de la biblioteca).
- Un dispositivo retirado (D71) o con valla (D84, D87) deja de ser reconocido, también si aún no se ha enterado.
- Qué hace de servidor: hoy un móvil solo escucha con la app abierta; ¿ambos lados pueden iniciar?
- Cómo convive con el emparejamiento por QR de hoy, que sigue siendo la puerta de entrada.

**Consecuencias:** cambia D13 y la sincronización local de [architecture/sync.md](architecture/sync.md). Hasta entonces, lo mitiga la sincronización cada 5 minutos con la app en primer plano y el reenvío de lo que llega por la red local (hito 7.3).
