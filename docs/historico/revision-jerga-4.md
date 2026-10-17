# Revisión de jerga · guía de usuario, cuarta tanda

> Revisión hecha el 17/10/2026 según `encargo-haiku-ultimas.md` (parte B). Lectura de dos páginas en `site/src/content/docs/es/using-the-app/`: `error-messages.md` y `whats-new.md`. No se ha cambiado ningún archivo. Los textos entre comillas que copian la app no se proponen cambiar: solo la explicación que los rodea.

Criterio: una persona de 60 años, que usa WhatsApp y el correo y no sabe nada de informática. Solo se apuntan las palabras o frases que no entendería o le harían dudar.

Términos que se repiten sin explicarse en estas dos páginas: **servidor**, **la nube**, **sesión**, **segundo factor**, **emparejar**, **ISBN**, **proveedor** y **clave** (en el sentido de contraseña de un servicio). Varias ya se explican en otras páginas, pero quien llega aquí desde un aviso no pasa por ellas. Conviene una frase de explicación en la primera aparición de cada una, o un enlace a la página donde se explican.

Además, en `error-messages.md` aparece «aparato» (líneas 89 y 92). En la guía, y por indicación del mantenedor del proyecto, va siempre «dispositivo» (o «móvil» / «ordenador»).

## error-messages.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| intro (párrafo 1) | «Ctrl+F» | Una combinación de teclas sin explicar; no todo el mundo sabe que es la búsqueda del navegador. | «Pulsa Ctrl+F (en el ordenador: la búsqueda de la página)». |
| intro (párrafo 2) | «sincronizar» | Palabra técnica que aparece en el texto sin explicar. | «pasar los cambios de un dispositivo a otro». |
| 17 | «Servicio» (en «No se pudo consultar *Servicio*») | Aquí «servicio» es un nombre de catálogo, pero la palabra suena a algo más amplio. | Dejarlo; si se quiere, «un catálogo (un servicio de internet)». |
| 18 | «catálogos online» | «Catálogo» sin explicar y «online» es inglés. | «en las bases de datos de libros de internet». |
| 19 | «Sin conexión con la nube» / «la nube» | «La nube» no se explica en esta página ni antes. | «la nube (tu cuenta guardada en internet)». |
| 20 | «Los cambios de la nube llegan al sincronizar» | Otra vez «nube» y «sincronizar» en la misma frase. | Explicar una vez en la fila 19 y aquí decir: «llegan cuando la app se pone al día». |
| 21 | «Este servidor no envía avisos al momento.» | «Servidor» y «avisos al momento» sin explicar. | «El servidor de tu cuenta no avisa a la app en el momento (tendrás que abrirla)». |
| 27 | «la dirección no es correcta» | «Dirección» puede entenderse como la de casa. | «la dirección de internet del servidor (la que empieza por https://)». |
| 28 | «`https://`» y «una que la app admita» | Lo escrito es código, sin decir qué es; «que la app admita» no dice cómo se sabe. | «Copia la dirección completa, la que empieza por https:// y no lleva nada detrás». |
| 29 | «no puede sincronizar con el servidor» | «Sincronizar» y «servidor» otra vez. | «no puede pasar los cambios con el servidor». |
| 31 | «Algo ha fallado con la nube.» | «Nube» otra vez, sin explicar. | Ver fila 19; no hace falta repetir la explicación. |
| 33 | «Demasiadas peticiones.» / «Has intentado muchas veces seguidas» | «Peticiones» es jerga de programador. La explicación de al lado sí se entiende. | Dejar el texto de la app y añadir: «Has pedido muchas cosas seguidas al servidor». |
| 49 | «tiene la sesión iniciada en otra cuenta» | «Sesión» no está explicado. | «tiene abierta la cuenta de otra persona». |
| 50 | «Tu cuenta conoce este dispositivo con otra clave.» | «Clave» aquí significa una clave técnica del dispositivo, no la contraseña; con «clave» en otras páginas (contraseña de un servicio), se confunde. | «Tu cuenta tiene guardado este móvil u ordenador de otra vez, con datos distintos». |
| 62 | «Segundo factor» (título) | Concepto que no se explica en la página. | «Segundo factor (un código extra al entrar, además de la contraseña)». |
| 66–70 | «app de autenticación» | Palabra técnica sin definir. | «la app de autenticación (una app del móvil que da códigos de 6 cifras)». |
| 68 | «código de respaldo» | Quien no lo recuerda no sabe qué es. | «un código de respaldo (uno de los diez que te dio la app al activar el segundo factor)». |
| 70 | «activación en curso» | Jerga; no se sabe qué se activa. | «Se cerró la pantalla de activar el segundo factor a medias». |
| 78 | «las demás sesiones» | «Sesiones» sin explicar. | «las demás veces que tienes la cuenta abierta». |
| 84 | «Emparejar dos dispositivos por la wifi» (título) | «Emparejar» no está explicado en la página. | «Conectar dos dispositivos (emparejar)». Una frase: «Emparejar es que dos móviles u ordenadores de tu casa se reconozcan entre sí». |
| 89 | «Estás leyendo el código en el mismo aparato que lo muestra.» | «Aparato» (debe ser «dispositivo»). | «Estás leyendo el código en el mismo dispositivo que lo muestra». |
| 90 | «Una VPN o el cortafuegos también pueden estorbar.» | «VPN» y «cortafuegos» sin explicar, y no le dicen nada. | «Si tienes activada una VPN (un servicio que oculta tu conexión), o el antivirus bloquea la app, desactívalos un momento». |
| 90 | «la red de invitados» | Jerga de router. | «la wifi de invitados (la que das a las visitas)». |
| 91 | «ha olvidado este dispositivo» | «Olvidar» no es lo que pone la app en su botón, que es «Desemparejar» (ver `forget` en `en.json`); la persona no sabe si es lo mismo. | Comprobar el nombre del botón en español y usarlo: «ha desemparejado este dispositivo». |
| 92 | «Otro aparato de la red ha contestado» | «Aparato» (debe ser «dispositivo»); «contestado» con «red» sin explicar. | «Otro dispositivo de tu wifi ha contestado en lugar del que esperabas». |
| 106 | «No tienes ese rol.» | «Rol» no está explicado en esta página. | «No tienes permiso para eso: solo lo tienen quienes administran la biblioteca». |
| 107 | «necesita personas con roles y que la nube pueda vincular dispositivos» | «Roles», «vincular» y «nube» sin explicar: son tres conceptos en una sola frase. | Partir en dos frases: «La biblioteca necesita a alguien con un papel asignado. Además, tiene que estar activada la opción de dar acceso a dispositivos nuevos desde la nube». |
| 111 | «Estás conectado a un servidor y el enlace es de otro.» | «Servidor» sin explicar. | «La app está conectada a un servidor, y el enlace es de otro distinto». |
| 113 | «subirla a la nube» | «Subir» y «nube» sin explicar en esta frase. | «guardarla en internet, en la nube». |
| 121 | «Un CSV se carga con **Importar CSV**.» | «CSV» es un formato de archivo; no lo entiende nadie que no sea informático. | «un archivo de hoja de cálculo (CSV)». |
| 124 | «Esta copia está protegida» | «Protegida» no dice que tiene contraseña en la misma frase (lo dice la explicación, bien). | Dejarlo. |
| 128 | «una versión de desarrollo que usaba otro formato» | «Versión de desarrollo» y «formato» sin explicar. | «una versión de pruebas de la app, que guardaba los datos de otra manera». |
| 139 | «Un ISBN tiene 10 o 13 dígitos.» | «ISBN» no se explica en la página. | «El ISBN es el número que traen los libros (10 o 13 cifras)». |
| 140 | «(y una X final)» | Un libro no sabe a qué se refiere la X. | «(puede acabar en una X, que es un número)». |
| 144 | «Otro libro del catálogo ya tiene este ISBN.» / «añade un ejemplar» | «Catálogo» y «ejemplar» sin explicar. | «Otro libro de tu biblioteca ya tiene este ISBN. Si es el mismo, añade una copia desde su ficha». |
| 145 | «Ya hay otra edición con este código de barras.» | «Edición» sin explicar. | «Ya hay otro libro (otra edición) con ese código de barras». |
| 147 | «*Servicio* ha rechazado la clave guardada.» / «Proveedores de datos» | «Clave» y «proveedores de datos» sin explicar en esta página. | Ver `data-sources.md` (fila 49 de su revisión): «la clave es como una contraseña que te da la web del servicio». |
| 148 | «cuota agotada» / «limita las consultas» | «Cuota» y «consultas» son jerga. | «Google ya ha hecho demasiadas consultas hoy, sin clave». |
| 152 | «abre una incidencia en el proyecto» | «Incidencia» es jerga de programador. | «escribe un aviso en la página del proyecto en internet (GitHub)». |

### Veredicto · error-messages.md

**Hay que reescribir partes.** La estructura (tablas por situación, «qué significa», «qué hacer») funciona. Los problemas se concentran en los términos técnicos que aparecen sin explicar: «nube», «sincronizar», «sesión», «segundo factor», «emparejar», «servidor», «ISBN», «clave» y «aparato». Con dos o tres definiciones en el inicio de la página (o un enlace a una página de glosario corto) se resuelve la mayoría.

## whats-new.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| intro (párrafo 2) | «Cuando una novedad cambia lo que sale de tu dispositivo» | «Lo que sale de tu dispositivo» es una frase abstracta. | «Cuando una novedad cambia lo que se envía desde tu móvil u ordenador a internet». |
| intro (párrafo 2) | «interruptor» | Se explica en el mismo párrafo: bien. | Dejarlo. |
| 14 | «Al emparejar un dispositivo hay que decir de quién es y con qué rol («editor» por defecto).» | «Emparejar» y «rol» sin explicar. | «Al conectar dos dispositivos (emparejar) hay que decir de quién es el otro y qué puede hacer: «editor» es lo normal». |
| 15 | «datos de prueba» / «la nube» / «unir la biblioteca con la original» | «Datos de prueba» se entiende, pero la frase es larga y tiene tres pasos. «La nube» no se explica y «unir» es ambiguo. | Partir en pasos numerados: 1) borra los datos de prueba en cada dispositivo; 2) en la nube, borra la cuenta y regístrate otra vez. Y aparte: «una copia restaurada en un móvil nuevo no se une a la original: es una biblioteca nueva con los mismos libros». |
| 16 | «la app de autenticación» | Palabra técnica sin definir (ver también `error-messages.md`, fila 66). | «la app de autenticación (la del código de 6 cifras)». |
| 17 | «ISBN con guiones o de 10 cifras» / «dígito de control» / «no cuadra» | «ISBN» sin explicar, y «dígito de control» es jerga. | «El ISBN es el número del libro. Si lo escribes mal, la app te avisa en vez de preguntarte qué es». |
| 18 | «BoardGameGeek» / «RPGGeek» / «el servidor de tu cuenta» / «Proveedores de datos» | Nombres de web sin contexto; «servidor» y «proveedores» sin explicar. | «BoardGameGeek (una web de juegos de mesa) y RPGGeek (una web de juegos de rol)». Y: «Lo que escribes va a tu cuenta de Kosmotheka, que lo consulta». |
| 22 | «la nube, opcional y por invitación» | Primera aparición de «nube», sin explicar. | «la nube (guardar tu biblioteca en internet, en tu cuenta), opcional y por invitación». |
| 24 | «buscar un libro por su ISBN» | «ISBN» sin explicar en esta página. | «buscar un libro por su número ISBN». |

### Veredicto · whats-new.md

**Pocos cambios.** Es una lista corta de novedades escritas con frases cortas, y casi todos los términos que faltan son los mismos que en `error-messages.md`. Con las definiciones que se añadan allí, basta con poner «nube», «ISBN» y «emparejar» la primera vez que aparecen en esta página, y cambiar la frase «lo que sale de tu dispositivo».

## Resumen

| Página | Veredicto |
|---|---|
| `error-messages.md` | Hay que reescribir partes (términos sin explicar en casi todas las tablas; «aparato» → «dispositivo»). |
| `whats-new.md` | Pocos cambios (una frase abstracta; definir «nube», «ISBN», «emparejar» y «rol» la primera vez). |

Lo que sigue sin tocar: los textos entre comillas que copian la app. Solo se proponen cambios en la explicación de alrededor.
