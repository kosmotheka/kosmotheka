# Revisión de jerga · guía de usuario, segunda tanda

> Encargo de `docs/historico/encargo-revision-jerga-2.md`, 17/10/2026. Solo lectura: no se ha cambiado ninguna página. Sin commit.

Lectora de referencia: una persona de 60 años que usa WhatsApp y el correo, sin conocimientos de informática. Se apunta lo que no entendería o le haría dudar, no el estilo. Las líneas son las del fichero, con el frontmatter incluido. Donde la app dice algo que no he podido comprobar (dónde aparece un texto, cuánto dura un aviso), lo marco «a confirmar».

## Patrones que se repiten

Estas son las cosas que más se repiten entre las seis páginas. Arreglarlas una vez en el glosario de la guía quita la mayor parte de las filas de abajo.

1. **«servidor»** sin explicar en `the-cloud`, `away-from-home` y `lost-phone`. Basta una frase la primera vez: «un ordenador de internet donde se guardan tus datos (el servidor)».
2. **«segundo factor», «códigos de respaldo», «app de autenticación»**: se usan en `the-cloud`, `lost-phone` y `delete-account` sin definir. Conviene un nombre llano («comprobación en dos pasos», «códigos de emergencia») y citar el nombre técnico una sola vez entre paréntesis.
3. **«emparejar» / «emparejado»** se usa en cuatro páginas y no se define en ninguna. Hace falta una frase: «conectar dos aparatos de casa para que se pasen los cambios».
4. **Tres nombres para un aparato**: «aparato», «dispositivo» y «móvil». «Dispositivo» aparece dentro de etiquetas de botón («Quitar un dispositivo», «Olvidar dispositivo»), así que hay que mantenerlo tal cual en esos botones, pero en el texto corrido conviene usar «aparato» y decirlo una vez.
5. **«rol» y «grupo de rol» / «Club de rol»**: en `remove-someone` (línea 21) y `people-and-permissions` (línea 10), «rol» de juego de rol choca con «rol» como permiso. Sugiero cambiar el ejemplo a un club de lectura.
6. **«administra» / «administrador»** se usa para la biblioteca y, en `lost-phone` (línea 51) y en `the-cloud` (línea 12), para quien mantiene el servidor. Son cosas distintas. Hay que decir «quien mantiene el servidor» en esos casos.
7. **«En directo», «avisos al momento», «Sin conexión en directo»**: tres nombres para lo mismo, y «en directo» evoca la televisión. Conviene elegir una forma y explicarla una vez.
8. **Notación «›» y «menú de tres puntos»** sin explicar. «Menú › Ajustes» se lee bien, pero la primera vez hay que decir qué es el menú. «Menú de tres puntos» (⋮) se usa en `remove-someone` y `lost-phone` sin decir cuál es ese icono.
9. **«copia de seguridad», «copia», «códigos de respaldo»**: tres formas para dos cosas distintas. Un solo nombre para cada una.
10. **Contradicción en `delete-account`**: la línea 16 dice que la cuenta «se borra al momento» y la línea 20 que se borra «en un plazo máximo de 30 días». Esto no es jerga, pero es lo que más puede asustar a esta lectora. Hay que aclararlo antes de publicar.

---

## the-cloud.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «La nube es **opcional**.» | Se usa «la nube» antes de decir qué es. | «La nube es guardar tus bibliotecas en internet, en un servidor, en vez de solo en tus aparatos. Es opcional:» |
| 12 | «En un servidor viven tus datos.» | «Servidor» sin explicar. | «Lo que subes a la nube se guarda en un ordenador de internet (el servidor).» |
| 12 | «quien administra ese servidor puede leerlo» | «Administra» aquí es de un servidor, no de la biblioteca. Se mezcla con el rol de administrador. | «quien mantiene ese servidor puede leer lo que guardas» |
| 12 | «lo guardado en el servidor no está protegido de modo que solo puedan abrirlo las personas de la biblioteca» | Una frase con doble negación y dos ideas. | «Quien mantiene el servidor puede leer lo que subes. Si no quieres que nadie más lo lea, no subas la biblioteca: la wifi de casa no pasa por ningún servidor.» |
| 13 | «Hoy el servidor oficial es por invitación.» | «Por invitación» y «hoy» no quedan claros: ¿tengo que tener una invitación? | «Ahora mismo, el servidor oficial solo deja crear cuenta a algunas direcciones de correo.» |
| 14 | «Solo sube una biblioteca quien la administra.» | «Administra» sin definir en esta página. | «Solo puede subir una biblioteca quien es su administrador (tiene ese rol).» |
| 15 | «Los avisos al momento (que el cambio de otro aparato llegue sin tocar nada) aún no están en el servidor oficial» | «Avisos al momento» es un nombre inventado para algo que no conoce. | «Por ahora, los cambios de otro aparato llegan cuando abres la app, no en el momento.» |
| 15 | «Mira «Sin conexión en directo» y «En directo» junto al estado.» | No sabe dónde está «el estado» ni qué quiere decir «en directo». | «Mira el texto que sale junto a la biblioteca: «En directo» o «Sin conexión en directo» (a confirmar dónde aparece en la app).» |
| 20 | «Menú › «Ajustes». En «Cuenta y nube», pulsa…» | «Menú» y «›» sin explicar. | «Abre el menú de la app (el icono de arriba), pulsa «Ajustes» y busca «Cuenta y nube».» |
| 23 | «caduca en 15 minutos» | «Caduca» es técnico y no dice qué pasa. | «…que solo vale durante 15 minutos.» |
| 23 | «tu correo y el idioma» | No se entiende qué idioma: ¿de la app o de los correos? | «tu correo y el idioma en que quieres recibir los correos (a confirmar)» |
| 24 | «Si usas un servidor que no es el oficial, pulsa «Otro servidor» y escribe su dirección.» | Solo lo necesita quien sabe que tiene otro servidor, pero la frase no lo dice. | «Solo si alguien te ha dicho que uses otro servidor, pulsa «Otro servidor» y escribe su dirección.» |
| 26 | «Al cambiarla, se cierra la sesión en tus otros aparatos» | «Sesión» y «cerrar la sesión» sin explicar; no sabe si pierde algo. | «Al cambiarla, los otros aparatos donde tenías la cuenta abierta se desconectan. No se borra nada: allí entrarás con la contraseña nueva.» |
| 28 | «Un candado más: el segundo factor» | «Segundo factor» no se entiende y el «candado» es una metáfora. | «Una comprobación extra al entrar, opcional. Su nombre técnico es «segundo factor».» |
| 30 | «una app de autenticación del móvil» | No sabe qué es ni cuál instalar. | «una aplicación de códigos del móvil (por ejemplo, Google Authenticator), que tienes que instalar antes.» |
| 33 | «Escanea el código con la app de autenticación y escribe el primer código que te dé.» | «Escanea el código» no dice con qué ni cómo; «el primer código» es ambiguo. | «Abre la aplicación de códigos, usa la cámara para leer el cuadrado con puntos (el código QR) que sale en pantalla y escribe el número de 6 cifras que te aparece.» |
| 34 | «10 códigos de respaldo. Solo los verás ahora» | «Códigos de respaldo» no dice para qué sirven. | «10 códigos de emergencia. Cada uno sirve una sola vez si pierdes el móvil. Solo se ven ahora: apúntalos en papel.» |
| 49 | «Si tiene cambios muy antiguos (de antes de cierta versión de la app), la app te propone hacer una copia» / «Hace falta una copia para subirla» | «Versión de la app» y «cambios muy antiguos» son términos internos; «copia» no dice que es una copia de seguridad. | «Si la biblioteca tiene datos guardados con una versión anterior de la app, la app te pide antes una copia de seguridad (ver Copias de seguridad).» |
| 51 | «La nube puede dar acceso a dispositivos nuevos»: encendido, el móvil nuevo de alguien de casa con cuenta entra solo, con el rol de esa persona. | Frase de dos ideas; «dispositivos» en lugar de «aparatos»; «rol» sin definir; «entra solo» ambiguo. | «“La nube puede dar acceso a aparatos nuevos”. Si está encendido, el móvil nuevo de alguien de casa que tenga cuenta entra solo, con el mismo permiso (rol) que esa persona tiene en la biblioteca. Si está apagado, das tú el acceso a mano.» |
| 51 | «vinculas a mano cada aparato nuevo» | «Vincular» no aparece en ninguna otra página; el resto dice «emparejar». | «das tú el acceso a mano a cada aparato nuevo (ver Compartir en casa)» |
| 57 | «Bajando tus bibliotecas de la nube» | «Bajando» es poco claro: no sabe si se descargan o se borran. | «Descargando tus bibliotecas de la nube.» |
| 62 | «Si quien administra dijo que ese aparato es de otra persona» | «Dijo» y el concepto de «dueño de un aparato» no se explican; «administra» otra vez. | «Si quien administra la biblioteca marcó que este aparato es de otra persona, la app te lo cuenta: «En “Casa”, este dispositivo es de Laura».» |
| 63 | «Solo baja lo que subiste.» | «Baja» es ambiguo. | «Al móvil nuevo solo llega lo que subiste.» |
| 72 | ««Por correo»: le llega un enlace; solo podrá usarlo con ese correo. El correo no se guarda.» | «Enlace» sin decir que es algo que se pulsa; «El correo no se guarda» no dice de qué correo habla. | «le llega un enlace (una dirección que se abre al pulsarla). Solo lo podrá abrir con ese correo. La app no guarda esa dirección (a confirmar).» |
| 73 | «Si cierras la ventana, no podrás volver a ver el enlace: cópialo antes (si no, anula la invitación y haz otra).» | «La ventana» no se sabe cuál es; «anula la invitación» es confuso. | «Cópialo antes de cerrar la pantalla: después no vuelve a aparecer. Si lo pierdes, pulsa «Anular» en «Personas» y haz otra invitación.» |
| 81 | «Hoy, tocar el enlace no siempre abre la app: pegarlo es lo seguro.» | «Hoy» fecha la guía; no se entiende por qué no abre la app; «pegarlo» sin explicar. | «Si al pulsar el enlace no se abre la app, cópialo y pégalo: mantén pulsado el cuadro de texto y elige «Pegar».» |
| 84 | «aún se puede dar acceso con un QR» | «QR» sin explicar en esta página; «el servidor» otra vez. | «aún se puede dar acceso con un código QR (el cuadrado con puntos que se lee con la cámara; ver Compartir en casa).» |

## remove-someone.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 12 | ««Personas y permisos» › «Quitar de…»» | La flecha «›» no dice que hay que entrar en un menú; el menú no está dicho. | «Menú, «Personas y permisos», y pulsa «Quitar de…»» |
| 13 | «Ese aparato deja de recibir las bibliotecas» | «Recibir las bibliotecas» no dice si se borran o se dejan de actualizar. | «Ese aparato deja de recibir cambios de esas bibliotecas.» |
| 14 | «Dejar de **emparejar** dos aparatos de casa» | «Emparejar» sin explicar. | «Dejar de conectar dos aparatos de casa para que no se pasen cambios» |
| 15 | «El móvil queda como recién instalado» | «Recién instalado» se entiende a medias. | «El móvil queda como si acabaras de instalar la app: sin bibliotecas ni cuenta.» |
| 21 | «Por ejemplo, alguien se ha ido del grupo de rol.» | «Grupo de rol» no se entiende y choca con «rol» como permiso. | «Por ejemplo, alguien ha dejado el club de lectura.» |
| 30 | «Sus aparatos borran la biblioteca la próxima vez que se conecten.» | «Se conecten» a nada en concreto. | «Sus aparatos borran la biblioteca la próxima vez que tengan internet o estén en la wifi de casa (a confirmar).» |
| 31 | «se conservan sus préstamos y lo que marcó» | «Lo que marcó» es vago: ¿leído, nota, favorito? | «se conservan sus préstamos y lo que ha marcado (si lo ha leído, su nota o si es favorito).» |
| 32 | «Lo que ya haya copiado fuera de la app» | «Copiado» se entiende, pero sin ejemplo no queda claro. Ya hay un ejemplo en la línea siguiente; sobra la frase larga. | «Lo que ya haya copiado o fotografiado fuera de la app no se puede quitar.» |
| 34 | «Antes, nombra a otra.» | «Nombrar» a alguien no se entiende como administrador. | «Antes, haz administradora a otra persona (cambiando su rol).» |
| 38 | «pero no se le devuelve nada por sí solo» | No se sabe si pierde o recupera algo. | «Vuelve a poder entrar, pero no recupera sus bibliotecas en sus aparatos por sí solo.» |
| 44 | «menú de tres puntos junto al aparato» | No sabe cuál es ese icono. | «el icono de tres puntos (⋮) al lado del aparato» |
| 46 | «Confirma con «Quitar» y el nombre del aparato.» | No se sabe si tiene que escribir el nombre. | «Confirma pulsando «Quitar». Si te pide el nombre del aparato, escríbelo (a confirmar).» |
| 48 | «lo que envíe no contará» | Abstracto y confuso. | «y los cambios que haga en él no se tendrán en cuenta.» |
| 54 | «Para volver a pasarse cambios habrá que emparejarlos otra vez» | «Emparejarlos» sin explicar; además «pasarse cambios» es informal. | «Para que vuelvan a pasarse cambios, habrá que conectarlos otra vez (ver Compartir en casa).» |
| 54 | «No se borra nada del catálogo.» | «Catálogo» en el sentido de la app; para ella puede ser el de una tienda. | «No se borra ningún libro.» |
| 56 | «Olvidar un aparato solo hace que deje de estar emparejado con este» | «Con este» no dice con cuál. | «Olvidar un aparato solo hace que deje de estar conectado con el móvil desde el que lo haces.» |
| 62 | «La app queda como recién instalada» | Igual que en la línea 15. | «Queda como si acabaras de instalar la app.» |

## people-and-permissions.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «Cada persona tiene un **rol** en cada biblioteca, y el rol decide qué puede hacer.» | «Rol» aparece antes de definirse. | «Cada persona tiene un rol (el papel que le das: qué puede hacer) en cada biblioteca.» |
| 10 | «alguien puede administrar «Casa» y ser lectora en «Club de rol»» | «Club de rol» choca con el significado de «rol» como papel; «administrar» como verbo sin pausa. | «alguien puede administrar «Casa» y ser lectora en «Club de lectura»» |
| 14 | «qué se hace con las peticiones» | «Peticiones» aparece antes de explicarse. | «qué se hace con las peticiones (los cambios que alguien pide)» |
| 15 | «borra solo lo que añadió su persona» | «Su persona» suena raro; no se entiende quién añadió. | «borra solo lo que añadió la propia persona que lo creó» |
| 24 | «cambiar si **puede compartir** la biblioteca con otros aparatos» | «Compartir» aquí es con aparatos, no con personas. | «cambiar si puede pasar la biblioteca a otros aparatos suyos» |
| 26 | «Un aparato que quien administra no ha dicho de quién es aparece en «Dispositivos sin persona» y **cuenta como lector**.» | «Cuenta como lector» no se entiende. «Dijo de quién es» es poco natural. | «Un aparato al que nadie ha asignado una persona aparece en «Dispositivos sin persona», y la app le da los permisos de lector.» |
| 26 | ««Asignar» y dice de quién es» | No dice a quién se asigna. | «pulsa «Asignar» y elige la persona a la que pertenece» |
| 34 | «Mientras esté pendiente puedes pulsar «Retirar».» | «Retirar» no dice que se cancela la petición. | «Mientras esté pendiente puedes pulsar «Cancelar».» (a confirmar el texto de la app) |
| 40 | «aparece una **campanita con un número**» | El icono no se describe. | «aparece un icono de campana con un número» |
| 45 | «la primera que decide gana; a la otra le aparece «Fulana ya la aceptó»» | «Gana» es raro para una decisión; «Fulana» parece un texto de plantilla. | «si dos personas administran, cuenta la primera que decide: a la otra le aparece «[nombre] ya la aceptó» y no tiene que hacer nada (confirmar que la app muestra el nombre real)» |
| 50 | «la app no te deja quitar a la última persona que administra ni su único aparato» | No se sabe si el «ni» habla de la persona o del aparato. | «la app no te deja quitar a la última persona que administra, ni quitarle su único aparato.» |
| 50 | «nombrar a otra persona administradora» | Mismo problema que en `remove-someone`. | «hacer administradora a otra persona» |
| 54 | «Si alguien copia a mano lo que ve (una foto de la pantalla, por ejemplo), la app no puede evitarlo.» | «Copiar a mano» con una foto no tiene sentido. | «Si alguien hace una foto de la pantalla o copia lo que ve, la app no puede impedirlo.» |
| 54 | «El rol decide qué se cambia, no qué se ve.» | Abstracto. | «El rol decide qué se puede cambiar, no qué se puede ver.» |

## away-from-home.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 12 | «abrir la **ficha** de cualquier ejemplar» | «Ficha» y «ejemplar» sin explicar. | «abrir la página de cualquier libro (y de cada uno de sus ejemplares, es decir, sus copias)» |
| 13 | «Añadir, editar y borrar, **a mano**.» | «A mano» da a entender escribir con la mano. | «Añadir, editar y borrar escribiendo tú los datos.» |
| 14 | «Apuntar lo que te falta» | «Lo que te falta» no se sabe si es una lista de deseados. | «Apuntar los libros que quieres tener (tu lista de deseados).» |
| 16 | «Saber si ya tienes un libro en una tienda («¿Lo tengo?»)» | «En una tienda» no dice qué tienda. | «Estando en una librería, saber si ya tienes ese libro («¿Lo tengo?»)» |
| 20 | «Buscar los datos de un libro (título, **autoría**, portada): los trae un servicio de internet.» | «Autoría» es poco usado; «un servicio de internet» es vago. | «Buscar los datos de un libro (título, autor, portada): la app los pide a una web de internet.» |
| 20 | «la búsqueda por **ISBN**, código o título» | «ISBN» es una sigla y «código» no dice qué código. | «la búsqueda por ISBN (el número que viene impreso en el libro, junto al código de barras) o por título» |
| 22 | «todo lo de la nube» | «La nube» no se explica en esta página. | «todo lo de la nube (ver La nube)» |
| 26 | «Los cambios se pasan entre aparatos **emparejados** cuando coinciden en la misma wifi con la app abierta» | «Emparejados» sin explicar; «coinciden» es formal. | «Los cambios pasan de un aparato a otro, si los has conectado antes, cuando están en la misma wifi y la app está abierta en los dos.» |
| 28 | «enciende en uno la «zona wifi» (compartir internet)» | El paréntesis lo explica mal; «zona wifi» es un término técnico. | «en uno de los dos móviles, activa la «zona wifi» (la opción que comparte tu internet con otro aparato)» |
| 28 | «no gasta datos de tu tarifa» | «Datos» y «tarifa» son jerga de compañía telefónica. | «no gasta los megas de tu tarifa» |
| 32 | «Junto al **estado** de la biblioteca verás una de estas cosas» | «Estado» no dice dónde mirar. | «En la biblioteca, junto a su nombre, verás una de estas frases (a confirmar dónde aparece):» |
| 34 | ««Sincronizado» y la hora» | «Sincronizado» es jerga. | ««Actualizado» y la hora» (a confirmar el texto de la app) |
| 35 | «no se llega al servidor» | «Servidor» otra vez. | «no hay internet o no se puede llegar a la nube.» |
| 36 | ««En directo» o «Sin conexión en directo»: si el servidor avisa al momento.» | «En directo» evoca la televisión; «avisa al momento» no se entiende. | ««En directo»: el servidor avisa en cuanto hay un cambio. «Sin conexión en directo»: no llegan avisos en el momento; los cambios de otros llegan cuando abres la app.» |
| 36 | «así que lo normal es ver «Sin conexión en directo»» | Parece una recomendación, no una descripción. | «así que, por ahora, lo normal es ver «Sin conexión en directo».» |
| 38 | «cuando se juntan gana el cambio más reciente de cada **dato**» | «Dato» y «gana» son técnicos; no sabe qué es un dato. | «cuando se juntan, queda el cambio más reciente de cada dato del libro (el título, la fecha…).» |
| 38 | «Si alguien borra algo que otra persona había editado, se borra.» | Dos ideas y la frase final no dice qué pasa con lo que se editó. | «Si una persona borra algo que otra había cambiado sin conexión, al juntarse desaparece para todos.» |
| 42 | «Sin servidor, no hay forma de ver tu biblioteca desde lejos.» | «Servidor» otra vez. | «Sin cuenta en la nube, no hay forma de ver tu biblioteca desde lejos.» |
| 42 | «Los cambios no viajan solos por internet entre dos aparatos si no hay cuenta.» | «Viajan solos» es una imagen confusa. | «Sin cuenta, los cambios no llegan solos por internet de un aparato a otro.» |

## lost-phone.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 14–15 | «En otro aparato **emparejado** (portátil, móvil de casa)» | «Emparejado» sin explicar. | «En otro aparato que tengas conectado a este (portátil, móvil de casa)» |
| 19 | «sin servidor, no hay ninguna copia de tu biblioteca en ningún sitio» | «Sin servidor» no se entiende. | «Si no tienes cuenta, no hay ninguna copia de tu biblioteca en internet.» |
| 27 | «empareja los dos o pásalas con una copia de seguridad» | «Pásalas con una copia» es ambiguo. | «o pasa las bibliotecas con una copia de seguridad (ver Copias de seguridad).» |
| 31 | «abre «Copias de seguridad» y **restaura** el archivo.» | «Restaurar» y «archivo» sin explicar. | «abre «Copias de seguridad» y elige el archivo de la copia: así recuperas las bibliotecas.» |
| 40 | «En otro aparato: Ajustes › tu nombre › «Mis dispositivos» › «Quitar» junto al perdido.» | Cadena de flechas; «Mis dispositivos» aparece sin contexto. | «En otro aparato, abre Ajustes, pulsa tu nombre, después «Mis dispositivos» y, junto al móvil perdido, «Quitar».» |
| 40 | «Hasta entonces puedes «Deshacer».» | No dice cuánto tiempo puede deshacerlo. | «Hasta entonces puedes pulsar «Deshacer» (a confirmar cuánto tiempo dura).» |
| 40 | «O cambia la contraseña: cierra la sesión en tus otros aparatos (no borra las bibliotecas de un móvil, pero ya no entrará con tu cuenta).» | Dos opciones y un paréntesis que mezcla qué se borra. | «O bien cambia la contraseña. Así se cierra la sesión en tus otros aparatos. Esto no borra nada del móvil perdido, pero ya no podrá entrar con tu cuenta.» |
| 42 | «en el menú de tres puntos del perdido, pulsa «Olvidar dispositivo»» | Icono de tres puntos sin explicar. | «en el icono de tres puntos (⋮) junto al móvil perdido, pulsa «Olvidar dispositivo»» |
| 51 | «los 10 códigos de respaldo a la vez y tenías el segundo factor» | Términos técnicos sin explicar. | «los 10 códigos de emergencia a la vez y tenías activada la comprobación en dos pasos (segundo factor)» |
| 51 | «Quien administra el servidor puede quitar el segundo factor de una cuenta» | «Administra el servidor» confunde con el rol de administrador. | «Quien mantiene el servidor puede quitar la comprobación en dos pasos de una cuenta» |

## delete-account.md

| Línea | Texto tal cual | Por qué no se entiende | Cómo lo dirías |
|---|---|---|---|
| 8 | «se borran al borrar la biblioteca o **desinstalar** la app» | «Desinstalar» es un término de móvil poco usado en la vida diaria. | «se borran al borrar la biblioteca o al quitar la app del móvil.» |
| 14 | «Confirma que eres tú: con tu contraseña (o, si entras con Google y no tienes contraseña, con un código que te llega al correo) y, si lo tienes activado, con el código del **segundo factor**.» | Frase larga con cuatro ideas; «segundo factor» sin explicar. | «Para confirmar que eres tú, escribe tu contraseña. Si entras con Google y no tienes contraseña, te llegará un código al correo. Si tienes activada la comprobación en dos pasos (segundo factor), escribe también su código.» |
| 16 | «Se borra al momento. La app te avisa antes de que se borra para siempre y de que se cerrará la sesión en tus aparatos.» | Se contradice con la línea 20 («en un plazo máximo de 30 días»), y «se borre» va con subjuntivo. | Aclarar cuándo se borra: «La app te avisa de que se borrará para siempre y de que se cerrarán las sesiones en tus aparatos» (a confirmar el plazo). |
| 27 | «La lista de tus aparatos conectados a la cuenta, y **su sesión**.» | «Su sesión» no se entiende. | «La lista de tus aparatos conectados a la cuenta, y que dejen de estar conectados.» |
| 32 | «pasan a otra cuenta que tenga acceso» | No se entiende quién las tiene después ni cómo se decide. | «siguen en la nube para las demás personas que las usan. [Aclarar quién queda como administrador: el texto no lo dice.]» |
| 33 | «El **registro de actividad** (qué se hizo, cuándo y desde qué **dirección IP**, sin el **contenido del catálogo**)» | Tres términos técnicos en una sola frase. | «Un historial técnico de lo que se hizo en el servicio (qué, cuándo y desde qué conexión de internet), sin los libros.» |
| 40 | «Menú › «Ajustes de la biblioteca» › «Borrar biblioteca».» | No queda claro si ese menú está dentro de la biblioteca o en la app. | «Dentro de la biblioteca, abre el menú y pulsa «Ajustes de la biblioteca», después «Borrar biblioteca».» |
| 41 | «**Todo el aparato**: «Ajustes» › «Dejar este dispositivo a otra persona»» | El enlace lleva a «Quitar a una persona o un aparato», y el título no dice que se borra todo. | «Todo lo del móvil: Ajustes, «Dejar este dispositivo a otra persona» (esto borra todo lo de ese móvil).» |
| 44 | «Más detalles en la **política de privacidad**.» | Término legal sin explicar. | «Más detalles en la política de privacidad (qué datos se guardan y para qué).» |

---

## Veredicto por página

- `the-cloud.md`: **hay que reescribir partes** (servidor, segundo factor, app de autenticación, códigos, «en directo», «vincular», bloque de invitaciones).
- `remove-someone.md`: **pocos cambios** (club de rol, «emparejar», menú de tres puntos, «recibir las bibliotecas»).
- `people-and-permissions.md`: **pocos cambios** («Club de rol», «Fulana», «cuenta como lector», «copia a mano»).
- `away-from-home.md`: **hay que reescribir partes** (el párrafo de sincronización de la línea 38, el bloque de «En directo», «zona wifi», «ISBN»).
- `lost-phone.md`: **pocos cambios** («restaurar», «sin servidor», la cadena de flechas de la línea 40, «Deshacer» sin tiempo).
- `delete-account.md`: **pocos cambios**, con una corrección de fondo (el plazo de borrado, línea 16 frente a línea 20) y la línea 33 para simplificar.

No he cambiado ninguna página. Este informe es el único fichero nuevo; no hay commit.
