# Encargo de diseño: personas, roles y peticiones (Fase 5)

> Tercer encargo para Claude Design, revisado por el maintainer el 27/09/2026. Las reglas de permisos están en D73 y los casos en `spec/conformance/draft/`. Lo decidido está en D45, D55, D56, D58 y D73 ([DECISIONS.md](../DECISIONS.md)). Todo en español, móvil (390 × 844) y escritorio (1280 × 800), claro y oscuro.

## Contexto

Cada biblioteca tiene ahora **personas con un rol**: administrador (todo, incluidos los permisos y las peticiones), editor (crea y edita; borra solo lo que creó) y lector (ve y pide). **Compartir es un permiso aparte.** Puede haber varios administradores. Cada dispositivo es **de una persona**: «el móvil de Hugo» es de Hugo.

Lo que alguien no puede hacer no se pierde: se convierte en **petición**, y los administradores la aceptan o la rechazan. Todo funciona sin servidor: cada dispositivo comprueba lo que le llega.

Hay que seguir las pantallas que ya existen, en especial «Compartir y dispositivos» (la tabla de escritorio y las tarjetas del móvil), «Ajustes de la biblioteca» y los diálogos de emparejar. En el menú ya están «Personas y permisos» y «Mis peticiones», marcados como «Pronto».

Una biblioteca **sin roles** funciona como hasta ahora; los roles empiezan cuando alguien asigna el primero. Las pantallas deben explicar ese paso sin asustar.

## Pantallas

### 1. Personas y permisos

La lista de personas de la biblioteca: nombre, rol, si puede compartir y sus dispositivos («Portátil», «Pixel 8»).
- Un administrador cambia el rol y el permiso de compartir. Los demás lo ven, sin poder cambiarlo.
- Estado vacío y primer paso: la biblioteca aún no tiene roles. Un texto corto que explique qué cambia al activarlos y un botón para empezar, que hace administrador a quien lo pulsa.
- Aviso cuando solo queda **un administrador**: conviene nombrar otro, porque si pierde su único dispositivo nadie podrá cambiar los permisos (D58, aún abierto).

### 2. ¿De quién es este dispositivo?

Al emparejar (el diálogo de escritorio y la pantalla de móvil que ya existen), un paso más: elegir a qué persona pertenece el otro dispositivo, o crear una persona nueva con su rol. En la tabla de dispositivos, cada uno muestra de quién es y se puede cambiar.

### 3. Peticiones

La bandeja de peticiones de la biblioteca:
- Para un administrador: qué se pide («Hugo quiere añadir *Tehanu*», «Laura quiere borrar un ejemplar de *Dune*»), quién y cuándo, y **Aceptar** o **Rechazar**. Ver el detalle de lo que cambiaría.
- Para quien pide: sus peticiones con su estado (pendiente, aceptada, rechazada).
- Contador en el menú («Mis peticiones · 2»).

### 4. Pedir en lugar de hacer

Donde un lector o un editor no puede hacer algo directamente, la acción cambia de nombre y de efecto:
- «Añadir» pasa a «Pedir que se añada» para un lector.
- «Borrar» pasa a «Pedir que se borre» para un editor, en lo que no creó su persona (D45).
- Aviso breve al enviarla («Petición enviada a los administradores»).

### 5. Cuando algo no entró

Un editor cambia algo sin conexión, pero mientras tanto le quitaron el permiso. Al sincronizar, su cambio no se aplica y pasa a petición. Hay que contárselo sin culpa, con el patrón de [errors.md](errors.md): qué pasó, por qué, qué hacer.

### 6. Dispositivo perdido

Desde «Personas y permisos», un administrador quita un dispositivo a una persona (se revoca en todas las bibliotecas que comparte), y le da acceso al nuevo.

## Qué entregar

Las pantallas de arriba en móvil y escritorio, claro y oscuro, en `templates/` como las anteriores, reutilizando sus piezas. Con Figtree y el acento añil de la marca, que ya están en la app.
