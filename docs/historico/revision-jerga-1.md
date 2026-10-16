# Revisión de jerga · guía de usuario (primera tanda)

> Informe del 16/10/2026, según el [encargo](encargo-revision-jerga-1.md). Solo lectura: no se ha cambiado ninguna página.
> Páginas revisadas: los siete ficheros de `site/src/content/docs/es/using-the-app/` (sin el `index.md`). Los números de línea son los del fichero, con el frontmatter incluido.

Criterio: una persona de 60 años, que usa WhatsApp y el correo y no sabe de informática. Se apuntan las palabras o frases que no entendería o que le harían dudar. No se apuntan faltas de estilo.

Términos que se repiten en varias páginas y conviene fijar antes de reescribir (los señalo en cada página):

- **«catálogo»** significa tres cosas: la lista de tus cosas, los servicios de internet que buscan datos de libros y la lista entera de la app.
- **«copia»**: a veces es una copia de seguridad y otras un ejemplar.
- **«fichero»** en vez de «archivo».
- **«proveedores», «catálogos», «bases de datos», «fuentes»**: se usan indistintamente para lo mismo.
- **«rol»**, **«permisos»**, **«personas»**: aparecen antes de explicarse.
- **«emparejar»**: es la palabra central de la página de compartir y no se define en ningún sitio.

---

## share-at-home.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «no hace falta cuenta, ni internet, ni ningún servidor» | «servidor» sin explicar (la página de qué-sale lo explica después, pero aquí no). | «ni pasa por ningún ordenador de internet» (o quitar la frase). |
| 8 | «Se hace con un código QR…» | «Emparejar» no se define en toda la página, y es lo que se hace. | «Emparejar es conectar dos aparatos para que se vean y se pasen los cambios. Se hace con un código QR…». |
| 15 | «Mejor si no es la red de invitados, porque en ella los dispositivos suelen no verse entre sí.» | «red de invitados» y «no verse entre sí» no se entienden sin conocer el router. | «Mejor no usar la wifi de invitados (la que tiene otra contraseña y se da a las visitas): en esa los aparatos no se ven entre ellos.» |
| 16 | «Sin VPN mientras emparejas. Una VPN es un programa que cambia por dónde sale tu conexión a internet…» | La explicación sigue siendo jerga, y la frase tiene dos ideas. | «Si tienes instalado un programa de «VPN» o de privacidad, apágalo mientras emparejas. Si no sabes qué es, no tienes que hacer nada.» |
| 24 | «Abre el menú y entra en «Compartir y dispositivos».» | No dice dónde está el menú. | «Abre el menú (los tres rayitas o los tres puntos de arriba) y entra en «Compartir y dispositivos».» (confirmar con la pantalla real). |
| 27 | «Verás un código QR y, debajo, el mismo código en texto.» | «en texto» es confuso. | «Debajo verás el mismo código escrito con letras y números.» |
| 34 | «Pulsa la pestaña «Escanear el suyo».» | «pestaña» es jerga de web. | «Pulsa la opción «Escanear el suyo».» |
| 36 | «apunta al código de A» | «apuntar» con el móvil es poco claro. | «pon la cámara delante del código de A». |
| 37 | «Pulsa «Copiar»… «Pegar su código»…» | «Pegar» sin explicar. | «Pulsa «Copiar»… y en B pulsa «Pegar su código» (es como copiar y pegar un texto)…». |
| 41 | «Recibidos 12 cambios, enviados 3» | No se entiende qué son «cambios». | «Se han pasado 12 cambios de Laura a este aparato (y 3 de este a Laura).» (o explicar qué es un cambio en una frase). |
| 45 | «Lo que se haga desde ese dispositivo contará como de esa persona, con sus permisos.» | «contará como de esa persona» y «sus permisos» son confusos. | «Lo que hagas desde ese aparato figurará como hecho por esa persona, y podrá hacer solo lo que le permita su rol.» |
| 46 | «También eliges su rol en cada biblioteca…» | «rol» aparece aquí por primera vez y no se define. | «También eliges qué puede hacer esa persona en cada biblioteca (su papel)…». |
| 48 | «Administrador: todo, incluidos los permisos y las peticiones.» | «permisos» y «peticiones» sin explicar. | «Administrador: puede hacerlo todo, también decidir quién entra y qué puede hacer cada uno.» («peticiones» no hace falta aquí). |
| 49 | «Editor: crea y edita; borra solo lo que añadió su persona.» | «su persona» no significa nada para quien no conoce el modelo. | «Editor: puede añadir y cambiar cosas, y borrar solo lo que ha añadido él o ella.» |
| 53 | «…también quién más forma parte de la biblioteca.» | «forma parte» es ambiguo. | «…también añadir o quitar personas de la biblioteca.» |
| 58 | «Los dos dispositivos se ponen al día solos… con la app a la vista…» | «ponerse al día» no se entiende del todo; «con la app a la vista» es poco claro. | «Los dos aparatos se actualizan solos cuando están en la misma wifi y la app está abierta en pantalla…». |
| 63 | «No gasta datos: todo va entre los dos aparatos.» | «datos» (del móvil) sin explicar; en esta página también aparece «datos del móvil» (línea 15). | «No gasta el megas de tu tarifa del móvil: todo va entre los dos aparatos.» |
| 62-63 | «activa el punto de acceso («compartir internet»)» | Muy técnico para esa persona. | «activa la opción de compartir internet del móvil (la «zona wifi»)…». |
| 71 | «Con «Olvidar dispositivo»… No se borra nada del catálogo.» | «olvidar» sugiere perder algo; «catálogo» con el tercer sentido. | «Con «Quitar este aparato»… tus libros no se borran.» (si el nombre de la pantalla es «Olvidar dispositivo», explicar en la frase que no borra nada). |
| 73 | «la app te dice que se lo pidas a quien la administra» | «administra» sin definir (sí se define en la página de primeros pasos, pero no aquí). | «…que se lo pidas a la persona que la gestiona en su aparato…» (o enlazar a [Primeros pasos]). |
| 77 | «no usan datos del móvil» | Igual que la línea 63. | «no usan los megas del móvil». |
| 79-80 | «Apaga la VPN, si usas una.» / «En Windows, si el programa de seguridad (el «cortafuegos») pregunta si deja pasar a la app, di que sí para las redes privadas.» | «cortafuegos» y «redes privadas» son técnicos. | «En Windows, si el antivirus o el firewall pregunta si deja entrar a la app, responde que sí.» |
| 81 | «Si el móvil pide permiso para la «red local» o los «dispositivos cercanos», acéptalo.» | Son textos de la pantalla del móvil, pero no se explica que son esos mensajes. | «Si el móvil pregunta si puede usar la «red local» (la wifi de casa) o los «dispositivos cercanos», acepta.» |
| 90 | «hará solo cuando se resuelva» | «se resuelva» es vago. | «lo hará solo en cuanto vuelvan a verse». |

Resultado: **hay que reescribir partes.** Es la página con más términos sin explicar (emparejar, roles, cambios, datos, VPN, cortafuegos). Conviene empezar por las líneas 8, 15, 45-49 y 58.

---

## backups.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «Conviene hacerlas de vez en cuando.» | Bien; sin cambio. | (ok) |
| 10 | «Todo esto está en el menú, en «Copias de seguridad».» | Igual que en la página de compartir: no dice dónde está el menú. | «En el menú (los tres rayitas de arriba), entra en «Copias de seguridad».» |
| 16 | «Una copia es un fichero con tus libros y sus portadas.» | «fichero» es jerga; y «copia» cambia de sentido en otras páginas. | «Una copia es un archivo con tus libros y sus portadas.» |
| 19 | «Viene así de entrada.» | «de entrada» es expresión coloquial pero suena a jerga. | «Ya viene activado.» |
| 22 | «una carpeta, un pendrive o tu espacio de Google Drive, por ejemplo.» | «pendrive» y «espacio de Google Drive» son términos que no todos usan. | «una carpeta, una memoria USB o tu Google Drive, por ejemplo.» |
| 26 | «Una app que guarde tus contraseñas es un buen sitio.» | «Una app que guarde tus contraseñas» (un gestor de contraseñas) sin explicar. | «Guárdala en un papel o en el gestor de contraseñas que uses.» |
| 28 | «Puedes quitar la contraseña apagando el interruptor. Entonces la app avisa de que quien encuentre el fichero podrá leerlo: nombres, lo que tenéis y quién tiene qué.» | «interruptor» y «fichero»; la frase tiene tres ideas. | «Puedes quitar la contraseña con el botón de activar/desactivar. La app avisa de que cualquiera que encuentre el archivo podrá verlo: los nombres, lo que tenéis y quién tiene qué.» |
| 32 | «Copia de todo el dispositivo» | «dispositivo» es el término de toda la guía (ya se usa así). Aquí se usa como sinónimo de «todas las bibliotecas». | «Copia de todas tus bibliotecas» (si es lo que hace). |
| 42 | «Si restauras en el mismo dispositivo que hizo la copia, la biblioteca vuelve como la misma. Si restauras en otro dispositivo, vuelve como una biblioteca nueva con los mismos libros, y **no se une a la original**.» | «vuelve como la misma» y «no se une» son confusos. La frase tiene muchas ideas. | «Si restauras en el mismo aparato, la biblioteca vuelve tal cual. Si lo haces en otro, aparece como una biblioteca nueva con los mismos libros, y **no se junta con la original**.» |
| 42 | «Para recuperar la biblioteca original en un dispositivo nuevo, empareja con otro dispositivo que la tenga… o bájala de tu cuenta, si usas una.» | «emparejar» sin definir; «cuenta» sin explicar; dos caminos en una frase. | «Para volver a tener la biblioteca original en un aparato nuevo, pásala desde otro aparato que la tenga (en [Compartir en casa]) o bájala de tu cuenta, si tienes una.» |
| 44 | «Si la app dice que la copia es de otra versión…» | «versión» sin explicar. | «Si la app dice que la copia se hizo con otra versión de la app…» (pequeño). |
| 46 | «el apartado «Hoja de cálculo»…» | No se explica qué es una hoja de cálculo. | «la parte «Hoja de cálculo», para pasar tu lista a Excel, LibreOffice u otro programa parecido…». |
| 52 | «Un CSV es un fichero de texto con una fila por ejemplar.» | «CSV», «fichero», «fila» y «ejemplar» (este último no se explica hasta la página de ejemplares). | «Un CSV es un archivo que Excel puede abrir, con una línea por cada ejemplar (cada copia física o digital de algo).» |
| 54 | «Tampoco lleva todo: no trae el tipo, las portadas, las series, tus marcas (leído, estrellas…) ni los datos propios de cada tipo.» | «tipo», «marcas», «series» y «datos propios de cada tipo» sin explicar. | «No lleva todo: no guarda las portadas, ni las series, ni lo que marcaste como leído o las estrellas.» |
| 58-85 | Tabla de columnas (`title`, `subtitle`, `authors`, `isbn13`…) y tabla de formatos en inglés | Es lo necesario para importar, pero la persona tendrá que escribir los nombres en inglés tal cual; «ISBN» no se explica aquí. | Mantener, pero dar una frase antes: «Los nombres de las columnas van en inglés, tal cual». Explicar el ISBN en una frase la primera vez. |
| 87 | «La app también acepta `single_issue`, `box`, `dvd`, `bluray`, `uhd`, `physical`, `digital`, `vinyl`, `cd` y `cassette`, pero la importación apunta todo como libro.» | Lista de palabras técnicas que no sirven para nada en esta importación; confunde. | Quitar la línea. |
| 89-93 | «Todo entra como libro.» / «se le añade un ejemplar» / «Las filas con errores se saltan…» / «con la línea de cada una» | «libro» contra el resto de tipos; «ejemplar»; «filas»; y «línea» (de la hoja o del texto) es ambiguo. | «Todo entra como libro.» (explicar que los demás tipos se cambian después a mano). «Si el ISBN ya está en tu lista, no se repite el libro: se le suma un ejemplar.» «Las filas con errores se saltan y la app dice cuáles.» |
| 93 | «cuántos ejemplares se han añadido a libros que ya tenías» | Ejemplar sin definir aquí. | Ver el caso anterior. |

Resultado: **hay que reescribir partes.** Dos cosas pesan más: la palabra «fichero» (aparece muchas veces) y la sección de restaurar, que tiene frases de tres ideas. La tabla de columnas es necesaria pero hay que avisar de que los nombres van en inglés.

---

## what-is-kosmotheka.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 12 | «…hay un tipo llamado «Otro».» | «tipo» sin explicar; el resto de la página lo usa. | «…hay una categoría llamada «Otro».» (o explicar en la frase «tipo» = «categoría»). |
| 19 | «En el ordenador, escribiendo el número.» | ¿Qué número? El ISBN aún no se ha explicado. | «En el ordenador, escribiendo el número que lleva el libro (el ISBN).» |
| 21 | «Por leer, leyendo, leído, abandonado, estrellas, favoritos y «Lo deseo».» | Es una lista de palabras sueltas; no se sabe a qué se refieren. | «Cada persona puede marcar su estado (por leer, leyendo, leído o abandonado), poner estrellas, marcar favoritos o «Lo deseo».» |
| 22 | «Tener varias bibliotecas.» | Bien: la idea se entiende con el ejemplo. | (ok) |
| 23 | «Compartirlas con quien vive contigo, por la wifi de casa…» | «wifi» está aceptado; sin cambio. | (ok) |
| 24 | «…traer o llevar tu lista como hoja de cálculo.» | «hoja de cálculo» sin explicar. | «…traer o llevar tu lista a un programa como Excel.» |
| 28 | «…para buscar los datos de un libro (título, autoría, portada…)» | «autoría» es una palabra poco usada. | «…para buscar los datos de un libro (título, autor, portada…).» |
| 28 | «Tus datos se guardan en tu dispositivo. La app funciona sin cuenta y sin conexión a internet.» | «dispositivo» (aquí sí se entiende si se explica). | «Tus datos se guardan en tu móvil u ordenador.» |
| 30 | «Hay también una cuenta opcional… Por ahora está en pruebas.» | «cuenta» sin explicar; «en pruebas» puede hacer pensar que no sirve. | «Hay también una cuenta opcional (como la de un correo), para tener tus bibliotecas en más aparatos aunque no estén en la misma wifi. Por ahora es una prueba. Sin cuenta, todo funciona igual.» |
| 32 | «Es software libre: su código está abierto para que cualquiera pueda leerlo.» | «software libre» y «código» son jerga; no aporta nada a esa persona. | «Es gratis, sin anuncios y sin seguimiento. Cualquiera puede ver cómo está hecha.» (o quitar la última frase). |

Resultado: **pocos cambios.** Casi todo se entiende. Lo que más pesa es «tipo» y la lista de palabras de la línea 21.

---

## first-steps.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 12 | «Android: Kosmotheka está en Google Play, pero por ahora solo en una prueba para un grupo reducido de personas.» | «Google Play» sin explicar; «en una prueba» no dice si puede instalarla. | «Android: la app está en la tienda de Google (Google Play), pero por ahora solo la pueden instalar unas pocas personas que participan en una prueba.» |
| 13 | «Ordenador (Windows y Linux): llegará pronto.» | «Linux» sin explicar. | «Ordenador con Windows: llegará pronto.» (si Linux no es para público general, quitarlo). |
| 21 | «Es el nombre que verán las personas con las que compartas tus bibliotecas. Con él se crea tu primera biblioteca, a tu nombre, y tú la administras…» | «administras» sin definir; dos ideas por frase. | «Es el nombre que verán las personas con las que compartas tus bibliotecas. Con él se crea tu primera biblioteca y eres quien la gestiona: puedes cambiarlo todo en ella.» |
| 21 | «Podrás cambiarlo después en «Ajustes».» | No dice dónde están los ajustes. | «Podrás cambiarlo después en «Ajustes» (en el menú).» |
| 22 | «de qué bases de datos de internet se sacan los datos de los libros (cada una con su interruptor)» | «bases de datos» y «interruptor» sin explicar. | «de qué servicios de internet se toman los datos de los libros (cada uno se puede encender o apagar)». |
| 25 | «Si más adelante quieres volver a ver la bienvenida, está en «Ayuda», con el botón «Ver la bienvenida».» | Correcto. | (ok) |
| 33 | «En el móvil es el botón de abajo a la derecha. En el ordenador está arriba a la derecha.» | Correcto. | (ok) |
| 34 | «Escribe el título, por ejemplo «Dune». Debajo van apareciendo resultados. Necesitas conexión a internet para esto.» | Correcto. | (ok) |
| 36 | «Lo único obligatorio es el título. Más abajo, en «Tu ejemplar»…» | «ejemplar» aparece aquí por primera vez y no se explica. | «Más abajo, en «Tu ejemplar» (cada copia que tienes de ese libro), puedes decir…». |
| 37 | «Verás el aviso «“Dune” se ha añadido al catálogo».» | «catálogo» (aquí significa «tu lista»). | «Verás el aviso «“Dune” se ha añadido a tu biblioteca».» |
| 43 | «Pulsa «Añadir a mano» y rellena tú los datos.» | Correcto. | (ok) |

Resultado: **pocos cambios.** Es la página más clara de la guía. Lo que hay que cambiar: la descripción de Google Play, «administras» y «catálogo» en la línea 37.

---

## add-a-book.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 10 | «La app no trae los datos de los libros de serie: los busca por ti en catálogos públicos de internet, como el de una biblioteca nacional.» | «de serie» significa otra cosa (por defecto); la frase no se entiende bien. «catálogos públicos» de nuevo. | «La app no lleva los datos de los libros guardados: los busca por ti en servicios públicos de internet, como el de la Biblioteca Nacional. Para eso necesitas conexión.» |
| 12 | «El ISBN es el número que lleva cada libro, junto al código de barras. Empieza por 978 o 979.» | Correcto. | (ok) |
| 22 | «Apunta al código de barras del libro. En los libros, es el que empieza por 978 o 979.» | Repite la línea 12. | (ok, o quitar la repetición) |
| 24 | «Si la tienda ha pegado una etiqueta propia encima del código…» | «etiqueta propia» es poco claro. | «Si la tienda ha puesto una pegatina encima del código de barras…». |
| 26 | «El escáner necesita un móvil con Google Play. Sin él, escribe el número a mano.» | «escáner», «Google Play» sin explicar. | «Para leer el código con la cámara necesitas un móvil Android con la tienda de Google (Google Play). Si no lo tienes, escribe el número a mano.» |
| 30 | «Escribe o pega el número en la caja de búsqueda. Un lector de códigos de barras USB, que escribe el número como si fuera un teclado, también sirve.» | «caja de búsqueda», «pega», «lector USB», «como si fuera un teclado» son demasiado técnicos. | «Escribe el número en el cuadro de búsqueda. (Si tienes un lector de códigos que se conecta al ordenador, también sirve; no es necesario.)» |
| 32 | «La caja reconoce números de 8, 12 o 13 cifras. Cuando lo reconoce, cambia su icono por el del código de barras.» | «caja» (cuadro) y «icono» ocupan lugar; el detalle de las cifras no hace falta. | «El cuadro reconoce el número cuando está completo y muestra un código de barras pequeño.» |
| 33 | «Si tu libro es antiguo, con un ISBN de 10 cifras o con guiones…» | Detalle técnico que confunde. | «Si el libro es antiguo y su número es más corto, pulsa «Añadir a mano»…». |
| 40 | «Cada resultado dice de qué catálogo viene.» | «catálogo» con otro sentido. | «Cada resultado dice de qué servicio viene.» |
| 46 | «Revísalos antes de guardar: los catálogos públicos a veces se equivocan…» | «catálogos públicos» otra vez. | «…porque los servicios de internet a veces se equivocan o dejan huecos.» |
| 48 | «…desde ahí puedes añadir otro ejemplar.» | «ejemplar» sin explicar aquí (se explica en la página siguiente). | «…desde ahí puedes añadir otra copia (un ejemplar).» |
| 53 | «sin cuenta ni clave propia la búsqueda solo se hace en Wikidata (una enciclopedia abierta de datos)…» | «clave propia» (la contraseña del servicio), y «Wikidata» está explicado. | «sin pedir ninguna contraseña propia, la búsqueda solo se hace en Wikidata (una enciclopedia abierta de datos)…». |
| 54 | «Hay catálogos que piden una clave propia: una contraseña gratuita… Se pone en «Ajustes», «Proveedores de datos».» | «clave propia» y «proveedores de datos» son términos nuevos. | «Hay servicios que piden una contraseña propia, gratuita, que te da su web. Se pone en «Ajustes», en «Servicios de datos».» (usar un solo nombre en toda la guía). |
| 58 | «el botón «Alta rápida»… sirve para escribir o escanear números uno tras otro… con los datos de ejemplar que hayas puesto debajo.» | «Alta» es jerga administrativa; «datos de ejemplar» sin explicar. | «el botón «Añadir muchos» sirve para meter números uno tras otro. Cada libro encontrado se guarda al momento con los datos de la copia que hayas puesto debajo.» |
| 64 | «Si tu rol es de lector» | «rol» otra vez sin explicar aquí (sí en la página de compartir). | «Si en esa biblioteca solo puedes consultar (eres lector)…». |

Resultado: **pocos cambios.** El problema es sobre todo de vocabulario: «catálogo», «clave», «proveedores», «caja» y «alta». Conviene unificar con las demás páginas.

---

## copies-and-places.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «Cada una de esas copias es un **ejemplar**.» | «copia» aquí no es una copia de seguridad (la de la página de copias). Confunde. | «Cada una de esas versiones que tienes es un **ejemplar**.» |
| 8 | «Un ejemplar dice en qué formato lo tienes…» | Correcto con la definición de arriba. | (ok) |
| 22 | «Además se pide la plataforma, que es la consola o el ordenador en el que se juega.» | Clara. | (ok) |
| 26 | «**Unidades.** Cuántas copias iguales tienes. Si tienes tres ejemplares idénticos, pon 3.» | **Contradicción**: si tres ejemplares idénticos se apuntan con 3 unidades, ¿cuándo se crea un ejemplar nuevo? La persona no sabe qué elegir. | Decidir y explicar: «Unidades: si tienes varias copias iguales en el mismo sitio (por ejemplo, tres del mismo libro en el salón), pon 3 en un solo ejemplar. Si están en sitios distintos o son de personas distintas, crea un ejemplar para cada una.» (confirmar con el comportamiento real). |
| 28 | «Con los formatos digitales (los que llevan una nube dibujada)…» | Depende de un icono que la página no muestra; no se sabe qué es «una nube dibujada». | «Con los formatos digitales (libros electrónicos, películas en internet…) no hay estantería: di dónde lo tienes, por ejemplo «Kindle de Laura».» (o añadir una captura del icono). |
| 36 | «Los datos del ejemplar al final de la ficha.» | Ficha no definida (ya aparece en primeros pasos «ficha» sin explicar). | «Los datos de la copia, al final de la ficha (la pantalla con la información del libro).» |
| 40 | «Abre la pestaña «Ejemplares».» | «pestaña» (de web). | «Abre la sección «Ejemplares».» |
| 47 | «cada ejemplar tiene un botón con tres puntos («Opciones del ejemplar»).» | Los tres puntos sí, pero «Opciones del ejemplar» es un texto de la app. | «cada ejemplar tiene un botón con tres puntitos («Opciones del ejemplar»)…». (ok en lo demás.) |
| 49 | «Si borras un ejemplar, el libro **sigue en el catálogo**…» | «catálogo». | «Si borras un ejemplar, el libro **sigue en tu lista**…». |
| 53 | «Lo que apuntas sin ningún ejemplar…» | «Me falta» y «Apuntar» se citan sin describir dónde están. | (ok) |
| 57 | «Que alguien sea propietario de un ejemplar no le da ningún permiso: los permisos son otra cosa, y se explican al [compartir en casa].» | «permisos» sin explicar; dos ideas. | «Que una persona sea la dueña de una copia no le da permisos para cambiar la biblioteca. Eso se explica en [Compartir en casa].» |
| 55-57 | «Las personas de la biblioteca» / «Personas y permisos» | Se nombra la pantalla «Personas y permisos» sin decir cómo se llega. | Añadir: «en el menú, «Personas y permisos»». |

Resultado: **pocos cambios**, pero la línea 26 (unidades) es un fallo de contenido: la persona no sabrá qué poner. Hay que decidir el comportamiento antes de reescribir.

---

## what-leaves-your-device.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «Tu catálogo se queda en tu dispositivo.» | «catálogo» con el sentido de «tu lista». | «Tus libros y demás cosas se quedan en tu móvil u ordenador.» |
| 12 | «…a catálogos públicos de internet…» | «catálogos públicos» otra vez. | «…a servicios públicos de internet…». |
| 14 | «Cada catálogo, como cualquier página web que visitas, ve desde qué conexión de internet le preguntas, y que la pregunta viene de Kosmotheka y de qué versión de la app.» | Una sola frase con cuatro ideas; «conexión de internet» no se entiende. | «Como cualquier página web, cada servicio ve desde qué dirección de internet le preguntas (la de tu casa). También ve que la pregunta viene de Kosmotheka y de qué versión de la app.» |
| 16 | «Estos catálogos están activados desde el principio.» | «activados» frente a «interruptor»: confuso. | «Estos servicios vienen encendidos desde el principio.» |
| 18-25 | Tabla: «Biblioteca Nacional de España», «Library of Congress», «Open Library», «Inventaire», «Wikidata», «MusicBrainz» | Nombres de servicios que la persona no conoce; «Inventaire» no se explica. | Una frase de explicación por servicio, o «Libros editados en España», etc. (ya está en la tabla), y quitar la palabra «Catálogo» de la cabecera. |
| 29 | «…GameUPC (juegos de mesa), TMDB (películas y series), RAWG (videojuegos) y Comic Vine (cómics).» | Siglas sin explicar (TMDB, RAWG). | «…y estos servicios, cada uno para un tipo de cosa: …» (explicar las siglas o quitarlas). |
| 29 | «Otros catálogos piden una **clave propia**…» | «clave propia». | «Otros servicios piden una **contraseña propia**: una que te da su web gratis para dejarte usar sus datos.» |
| 31 | «**Dónde cambiarlo.** Entra en «Ajustes», «Proveedores de datos». Ahí ves, tipo por tipo…» | «proveedores de datos» otra vez (distinto de «catálogos» y de «bases de datos»). «tipo por tipo». | «Entra en «Ajustes», «Servicios de datos» (debe llamarse igual en todas las páginas). Ahí ves, para cada tipo de cosa, qué servicios se usan…». |
| 35 | «La imagen de la portada se baja desde el catálogo que la tiene, y se guarda en tu dispositivo. Esa descarga también llega a ese catálogo.» | «catálogo»; «esa descarga también llega a ese catálogo» es confuso (¿qué significa «llegar»?). | «La imagen de la portada se baja de internet desde el servicio que la tiene y se guarda en tu móvil u ordenador. Ese servicio sabe que la has pedido.» |
| 39 | «Las imágenes se procesan en el propio dispositivo: no se guardan ni se envían a ningún sitio.» | «procesan» es jerga. | «Las imágenes se leen en tu propio aparato: no se guardan ni se envían a ningún sitio.» |
| 41 | «la app usa un componente de Google que viene con Google Play. Sus condiciones están en «Acerca de», «Licencias».» | «componente» y «Google Play» sin explicar. | «la app usa una pieza de software de Google (que viene con la tienda Google Play). Puedes leer sus condiciones en el menú, en «Acerca de» y «Licencias».» |
| 45 | «Cuando emparejas dos dispositivos, los datos van **directamente de uno a otro, por tu wifi**.» | «emparejas» sin definir (ver la página de compartir). | «Cuando conectas dos aparatos para compartir (en [Compartir en casa]), los datos van **directamente de uno a otro, por tu wifi**.» |
| 47 | «…ven todo lo que hay en ella, incluidos los nombres de las personas y de quién es cada cosa.» | Correcto. | (ok) |
| 51 | «Si los subes a un servicio de internet (Google Drive, por ejemplo), se aplican las normas de ese servicio.» | «normas» es vago. | «…se aplican las condiciones de ese servicio (Google Drive, por ejemplo).» |
| 51 | «La copia de seguridad va protegida con contraseña; el CSV, no.» | «CSV» sin explicar en esta página. | «…la copia de seguridad va con contraseña; la lista en hoja de cálculo (CSV), no.» |
| 55 | «…y las búsquedas de algunos catálogos pueden hacerse a través de él.» | «catálogos» y frase larga con dos ideas. | «…y las búsquedas de algunos servicios pueden pasar por ese ordenador.» |
| 55 | «Qué guarda el servidor, durante cuánto tiempo y cómo borrarlo está en la [política de privacidad].» | «servidor» (explicado en la línea anterior, en parte); «política de privacidad» es jerga. | «Qué guarda ese ordenador, durante cuánto tiempo y cómo borrarlo está en [la página de privacidad].» |
| 66 | «Borrar una biblioteca la quita de ese dispositivo (y de los demás con los que la compartes).» | **Ambigua y de riesgo**: parece que borrarla en un aparato la borra en todos. Hay que confirmar el comportamiento real y decirlo sin dejar lugar a dudas. | Si es así: «Borrar una biblioteca la quita de este aparato, y también de los demás con los que la compartes. Si quieres conservarla en otro aparato, haz antes una copia de seguridad.» (confirmar). |
| 66 | ««Dejar este dispositivo a otra persona», en «Ajustes», lo deja como recién instalado.» | «recién instalado» es claro; «dispositivo» otra vez. | «…lo deja como cuando se compró o se instaló la app por primera vez.» (ok en lo demás). |

Resultado: **pocos cambios.** La línea 66 hay que comprobarla con el comportamiento real antes de publicar, porque puede causar un borrado no esperado. La línea 14 es la frase más larga de la página.

---

## Resumen

| Página | Veredicto |
|---|---|
| `what-is-kosmotheka.md` | pocos cambios |
| `first-steps.md` | pocos cambios |
| `add-a-book.md` | pocos cambios |
| `copies-and-places.md` | pocos cambios (ojo: línea 26, unidades, contradice el texto) |
| `what-leaves-your-device.md` | pocos cambios (ojo: línea 66, borrar una biblioteca) |
| `share-at-home.md` | hay que reescribir partes |
| `backups.md` | hay que reescribir partes |

## Lo que pide decidir antes de reescribir

1. **«Catálogo»**: usar «tu lista» para lo que es tuyo, y «servicios de internet» para las fuentes de datos. Hoy aparece en muchas líneas de las siete páginas con esos tres sentidos.
2. **«Proveedores de datos» / «catálogos» / «servicios»**: un solo nombre para el menú «Ajustes» y para las fuentes.
3. **«Fichero»** → «archivo». Aparece en casi todas las páginas.
4. **«Copia»**: reservarla para la copia de seguridad, y decir «ejemplar» para cada libro físico o digital.
5. **«Emparejar»**: definirla una vez, en la página de compartir, y enlazarla desde las demás.
6. **Unidades (copies-and-places, línea 26)**: el texto dice dos cosas que se contradicen. Hay que confirmar el comportamiento de la app.
7. **Borrar una biblioteca (what-leaves-your-device, línea 66)**: confirmar si se borra también en los demás aparatos.
