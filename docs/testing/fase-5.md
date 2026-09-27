# Pruebas manuales de la Fase 5

> Mismo formato que `hito-1.md`: pasos, resultado esperado, `[x]` al pasar. Las firmas tienen tests automáticos (tres dispositivos, manipulación en el camino, suplantación, cambio de clave) y casos compartidos con PHP. Aquí va lo que se comprueba con la app delante.

## Antes de empezar

La primera vez que abras esta versión, la app migra la base de datos (migración 11: la tabla `devices` y la firma de cada operación) y deja una copia al lado: `library.sqlite3.v10.bak`. Si algo sale mal, cierra la app, borra `library.sqlite3` y renombra la copia.

**Todos los dispositivos tienen que tener esta versión** para sincronizar: con versiones distintas, la sincronización se niega y lo dice (como hasta ahora).

## 5.1 · Firmas

No se ven en pantalla: si todo va bien, nada cambia. Lo que hay que comprobar es que sigue funcionando todo lo de antes.

- [ ] **Lo de siempre sigue ahí.** Abre la app tras actualizar. *Esperado:* los mismos libros, bibliotecas e historial.
- [ ] **Añadir y sincronizar.** Con dos instancias (ver `fase-2.md`), añade un libro en una y sincroniza. *Esperado:* llega a la otra, como antes; en el resumen, 0 rechazados.
- [ ] **A través de un tercero.** Con tres instancias, A empareja con B y B con C (A y C nunca se emparejan). Añade un libro en A, sincroniza B con A y C con B. *Esperado:* el libro llega a C.
- [ ] **El móvil.** Instala el APK en el móvil y sincroniza con el portátil. *Esperado:* funciona igual que antes.

## 5.2 a 5.7 · Personas, permisos y peticiones

Estas versiones migran la base de datos otra vez (migraciones 12 a 15: roles, peticiones con nota y dueños pendientes) y dejan su copia al lado, igual que arriba.

**Preparar tres instancias** (ver «Preparar dos instancias» en `fase-2.md`, con una tercera carpeta de datos). En lo que sigue:
- **A** es el portátil de Ana, que administrará «Casa».
- **B** es el móvil de Laura, que editará.
- **C** es el móvil de Hugo, que solo leerá.

Pon nombre a cada instancia en Compartir y dispositivos → «Nombre de este dispositivo» («Portátil», «Móvil de Laura», «Móvil de Hugo»): es lo que verán las demás.

### Activar los permisos

- [ ] **Sin permisos, todo como antes.** En A, empareja B y C compartiendo «Casa», y sincroniza. Añade un libro desde C. *Esperado:* se añade sin pedir nada, como hasta ahora.
- [ ] **Explicación antes de activar.** En A → Personas y permisos. *Esperado:* una pantalla explica qué son los roles y ofrece activarlos; no cambia nada hasta seguir.
- [ ] **¿De quién es cada dispositivo?** Sigue → escribe «Ana» como tu nombre → B: «Persona nueva» «Laura», Editor → C: «Persona nueva» «Hugo», Lector → activa. *Esperado:* la tabla muestra a Ana (Administrador), Laura (Editor) y Hugo (Lector), cada uno con su dispositivo.
- [ ] **Llega a los demás.** Sincroniza B y C. *Esperado:* en B y C, Personas y permisos muestra lo mismo, en solo lectura, con la nota de que lo decide quien administra.
- [ ] **Aviso de una sola administradora.** En A. *Esperado:* un aviso dice que Ana es la única que administra, con «Nombrar a otra persona». Su menú de rol no deja elegir otro.

### Roles

- [ ] **La editora edita sin pedir.** En B, añade un libro y edita otro. *Esperado:* se hace al momento, sin petición.
- [ ] **La editora borra lo suyo.** En B, borra el libro que acaba de añadir. *Esperado:* se borra.
- [ ] **… y pide borrar lo de otros.** En B, abre un libro añadido desde A → menú ⋮. *Esperado:* en vez de «Borrar» dice «Pedir que se borre», con «Lo añadió otra persona: lo decide quien administra Casa». Pídelo con una nota. *Esperado:* «Petición enviada a quien administra».
- [ ] **El lector no ve editar.** En C, abre un libro. *Esperado:* no hay «Editar» ni «Añadir ejemplar»; el menú dice «Pedir que se borre».
- [ ] **El lector pide añadir.** En C, la barra superior (o el botón flotante en el móvil) dice «Pedir que se añada». Rellena un libro, escribe una nota y «Enviar petición». *Esperado:* el aviso dice que se envió; el libro no aparece en el catálogo de C.
- [ ] **… también desde ¿Lo tengo?** En C, busca un ISBN que no tengáis. *Esperado:* el botón dice «Pedir que se añada» y abre el mismo formulario con los datos.
- [ ] **El lector apunta deseos.** En C, «Me falta» → apunta un deseo. *Esperado:* se apunta sin pedir nada. Borrarlo también funciona; borrar uno que apuntó Ana, no.
- [ ] **Mis peticiones.** En C → menú → «Mis peticiones». *Esperado:* sus peticiones pendientes, con «Retirar».

### Peticiones

- [ ] **La campanita.** Sincroniza A con B y C. *Esperado:* en A aparece una campana con el número de peticiones, y el mismo número junto a «Peticiones» en el menú.
- [ ] **Qué cambiaría.** En A → Peticiones. *Esperado:* a la izquierda, «Hugo quiere añadir …» y «Laura quiere borrar …»; al elegir una, a la derecha, «Qué cambiaría si aceptas» con los datos y la nota.
- [ ] **Aceptar.** Acepta la de Hugo. *Esperado:* el libro aparece en el catálogo de A; la petición pasa a «Resueltas». Sincroniza C: el libro aparece y en «Mis peticiones» sale «Aceptada · Ana».
- [ ] **Editar y aceptar.** Pide otro libro desde C con una errata en el título, sincroniza, y en A → «Editar y aceptar». *Esperado:* se abre el formulario relleno; corrige y guarda. Entra corregido.
- [ ] **Rechazar.** Rechaza la de Laura. *Esperado:* el libro sigue; en B, «Mis peticiones» dice «Rechazada».
- [ ] **Retirar.** En C, pide algo y retíralo antes de sincronizar. Sincroniza. *Esperado:* en A no aparece como pendiente; sale como «Retirada».
- [ ] **Ya no se puede aplicar.** Desde B pide borrar un libro de Ana; antes de aceptarlo, Ana borra ese libro en A. *Esperado:* la petición avisa de que ya no se puede aplicar y «Aceptar» está desactivado.
- [ ] **Resuelta por otra persona.** Con dos administradores en dos dispositivos, deja la misma petición abierta en los dos, acepta en uno, sincroniza y acepta en el otro. *Esperado:* «Laura ya la aceptó», con «Entendido».
- [ ] **Un cambio tuyo ha pasado a petición.** En A, baja a Laura a lectora, sin sincronizar todavía. Después, en B, cambia la ubicación de un libro (B aún se cree editora y lo hace). Sincroniza B con A. Cuenta el orden en que se hicieron, no el de llegada: el cambio de B fue posterior a la bajada. *Esperado:* B avisa de que un cambio suyo ha pasado a petición, con «Ver mi petición».

### Dispositivos y emparejar

- [ ] **De quién es.** En A → Compartir y dispositivos. *Esperado:* una columna «De quién es» (en el móvil, «Es de») con la persona de cada dispositivo; «Cambiar» la cambia. En B y C se ve igual, sin poder cambiarla.
- [ ] **Emparejar diciendo de quién es.** En A, empareja una cuarta instancia **D** compartiendo «Casa». *Esperado:* tras conectar, un paso más: «¿De quién es …?». Elige a Laura. *Esperado:* «Casa · Editor · ya lo tenía»; «Dar acceso a Laura». Sincroniza D: Personas y permisos muestra el dispositivo en Laura.
- [ ] **El rol no se hereda.** Empareja otra vez compartiendo también una biblioteca con roles donde Laura no esté. *Esperado:* en esa biblioteca pide elegir el rol, sin ninguno marcado, y «Dar acceso» no se activa hasta elegirlo.
- [ ] **Persona nueva.** Empareja eligiendo «Otra persona». *Esperado:* pide nombre y rol en cada biblioteca.
- [ ] **En el móvil, en pasos.** Empareja desde el móvil de alguien que administra. *Esperado:* «Paso 2 de 3» con la lista de personas y «Siguiente»; el paso 3 (roles) solo aparece si falta alguno.
- [ ] **Ahora no.** Empareja y pulsa «Ahora no». *Esperado:* el dispositivo queda «Sin persona · cuenta como lector», y se puede asignar después.
- [ ] **Sin permiso para compartir.** En B (Laura, sin «Puede compartir») → Compartir y dispositivos. *Esperado:* «En Casa no puedes compartir; pídeselo a quien la administra»; al emparejar, «Casa» no se ofrece. En A, activa «Puede compartir» para Laura y sincroniza B: ya se ofrece.

### Dispositivo perdido

- [ ] **Quitar un dispositivo.** En A → Personas y permisos → Hugo → quitar el móvil de Hugo. *Esperado:* explica qué bibliotecas pierde. Sincroniza C: esas bibliotecas desaparecen de C.

## 5.8 · Marcas por persona

Esta versión añade la migración 16 (tabla `marks`) y deja su copia al lado, como las anteriores.

### Sin roles

Con una biblioteca sin permisos activados, y dos instancias **A** y **B** que la comparten.

- [ ] **¿Quién eres?** En A, abre un libro y toca «Terminado» en «Lo mío». *Esperado:* sale «¿Quién eres?» con las personas que ya tienen ejemplares (y cuántos) y «Otra persona». Elige una. *Esperado:* el libro queda terminado con la fecha de hoy; la biblioteca sigue sin permisos (Personas y permisos no cambia).
- [ ] **No se vuelve a preguntar.** Marca otro libro en A. *Esperado:* se marca sin preguntar.
- [ ] **Estado, fecha, estrellas y favorito.** Cambia a «Leyendo». *Esperado:* «Desde el …» con la fecha de hoy. Toca la fecha y elige otra. Pon 4 estrellas y «Favorito». Toca otra vez la cuarta estrella. *Esperado:* se quitan las estrellas.
- [ ] **Quitar.** Toca «Quitar». *Esperado:* «Lo mío» vuelve a estar vacío. Marca de nuevo. *Esperado:* se puede marcar otra vez sin problema.
- [ ] **Lo de los demás.** En B, di quién eres (otra persona) y marca el mismo libro. Sincroniza. *Esperado:* en A sale una línea «… lo terminó en …» bajo «Lo mío» que se despliega con cada persona; en B, lo mismo con lo de A.
- [ ] **Misma persona en dos dispositivos.** En B, di que eres la misma persona que en A. Sin sincronizar, pon «Terminado» en A y 5 estrellas en B en el mismo libro. Sincroniza. *Esperado:* en los dos, una sola marca con «Terminado» y 5 estrellas.

### En el catálogo

- [ ] **Indicador.** *Esperado:* junto al título de lo que has marcado, un icono pequeño con tu estado y un corazón si es favorito; nada en lo que no has marcado. En escritorio aparece en cuanto marcas algo en el panel.
- [ ] **Filtrar.** «Filtros» → «Mi estado» → «Terminado». *Esperado:* solo lo que has terminado, con la etiqueta «Mi estado: Terminado» para quitarlo. Añade «Sin marcar». *Esperado:* también lo que no has marcado.
- [ ] **Mis favoritos.** Activa «Solo mis favoritos». *Esperado:* solo tus favoritos; «Quitar filtros» lo deja todo como estaba.

### Con roles

- [ ] **Cada persona marca lo suyo.** En la biblioteca con roles de las pruebas de arriba, en C (Hugo, lector), marca un libro. *Esperado:* se marca sin pedir nada, aunque sea lector.
- [ ] **Dispositivo sin persona.** En un dispositivo que no es de nadie. *Esperado:* «Lo mío» sale desactivado con «alguien que administra … tiene que decir antes de quién es este dispositivo»; no pregunta «¿Quién eres?».
- [ ] **Borrar el libro.** Borra un libro marcado y sincroniza. *Esperado:* desaparece con sus marcas en todos los dispositivos, sin pasar a petición por las marcas de otras personas.
