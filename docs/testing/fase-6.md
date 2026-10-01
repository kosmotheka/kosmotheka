# Pruebas manuales de la Fase 6

> Mismo formato que `fase-5.md`: pasos, resultado esperado, `[x]` al pasar. El motor tiene tests automáticos (tipos, detalles por tipo, series, géneros, etiquetas, marcas, convergencia entre dispositivos) y casos compartidos con PHP. Aquí va lo que se comprueba con la app delante; la prueba de verdad es con la familia, sobre todo si se entiende.

## Antes de empezar

La primera vez que abras esta versión, la app migra la base de datos (migraciones 17 a 25: tipos, código de barras, detalles por tipo, «lo deseo», géneros y etiquetas, series, plataforma y temporadas en el ejemplar, notas, y fuera la lista de deseos antigua) y deja una copia al lado (`library.sqlite3.v16.bak` o la versión que tuvieras). Si algo sale mal, cierra la app, borra `library.sqlite3` y renombra la copia.

**Todos los dispositivos tienen que tener esta versión** para sincronizar.

- [ ] **Lo de antes sigue ahí.** *Esperado:* los mismos libros, ejemplares, marcas e historial; todo es «Libro».
- [ ] **Una copia de seguridad antigua se restaura.** Restaura una copia hecha con la versión anterior. *Esperado:* se restaura entera.

## 6.1 y 6.2 · Tipos de obra

- [ ] **Catálogo mezclado.** Añade a mano una obra de cada tipo. *Esperado:* chips de tipo arriba («Todo», «Libros», «Películas»…); cada portada sin imagen tiene la forma de su tipo (caja de DVD con lomo, caja de videojuego con franja, cuadrada para juegos de mesa y música).
- [ ] **«¿Qué es?».** Añadir a mano empieza preguntando el tipo (en móvil, una hoja; en escritorio, una fila de ocho). *Esperado:* el formulario cambia su parte «Datos de…» según el tipo, sin perder lo escrito.
- [ ] **Personas por papel.** En un cómic, pon guion y dibujo; en un juego, diseño. *Esperado:* chips con autocompletado; en la ficha salen con su papel.
- [ ] **Ejemplar de cada tipo.** Añade un ejemplar a un videojuego. *Esperado:* primero la plataforma (Switch, PS5…), luego físico o digital; en digital, «dónde» propone tiendas (eShop, Steam). En una serie: «Qué temporadas».
- [ ] **Estados según el tipo.** En una película, «Lo mío» dice «Por ver · Viendo · Vista · Abandonada»; en la lista mezclada, los genéricos.
- [ ] **Cambiar de tipo.** Edita un libro con un ejemplar en bolsillo y cámbialo a cómic. *Esperado:* aviso con lo que pasa (autoría a guion, ISBN a código de barras) y lo que se pierde (idioma); el bolsillo pide elegir formato y no deja confirmar hasta elegirlo.
- [ ] **Notas.** Escribe notas en una obra. *Esperado:* se ven en la ficha y se sincronizan.

## 6.3 · El alta empieza por buscar

- [ ] **Buscar por título.** «Añadir» → Libro → escribe «dune». *Esperado:* resultados de Open Library, Inventaire y Wikidata, cada uno con su fuente; si ya lo tienes, «Ya está en … · añadir un ejemplar».
- [ ] **Elegir un resultado.** *Esperado:* el formulario se abre relleno (título, autoría, año, portada) y se puede volver a buscar.
- [ ] **Películas, juegos, videojuegos.** Busca una película. *Esperado:* resultados de Wikidata y el aviso discreto de que ese tipo busca poco sin nube ni clave.
- [ ] **Escanear un ISBN** (móvil). *Esperado:* elige «Libro» solo y muestra lo encontrado.
- [ ] **Escanear un disco** (móvil). *Esperado:* lo reconoce como música (MusicBrainz) con el formato sugerido.
- [ ] **Un código desconocido.** *Esperado:* «¿Qué es?»; al elegir tipo se abre el formulario con el código ya puesto.
- [ ] **Sin conexión.** Busca sin red. *Esperado:* un aviso de que no se pudo buscar y «Añadir a mano» funciona.

## 6.4 · Proveedores

- [ ] **Ajustes › Proveedores de datos.** *Esperado:* cada tipo con sus fuentes y «N de M»; en ámbar los que buscan poco.
- [ ] **Apagar una fuente.** Apaga Inventaire y busca un libro. *Esperado:* ya no salen resultados de Inventaire.
- [ ] **Clave propia** (si tienes una de Google Books o GameUPC). Pégala, «Guardar» y «Probar». *Esperado:* «Probar» dice si funciona; una clave mala avisa en las búsquedas.

## 6.5 · Me falta y lo deseo

- [ ] **Apuntar lo que falta.** En «Me falta», «Apuntar» (móvil) o «Añadir» (escritorio) y elige un resultado. *Esperado:* el formulario no pide ejemplar; la obra sale en «Me falta» y no en «Lo tengo».
- [ ] **Lo tengo.** En la ficha de una obra que falta, «Lo tengo». *Esperado:* se abre el formulario de ejemplar; al guardarlo pasa a «Lo tengo».
- [ ] **Lo deseo.** Marca «Lo deseo» en una obra. *Esperado:* en la ficha, «Lo deseas tú»; en otro dispositivo de otra persona, «Lo desea <tu nombre>» y aparece en «Lo que desean otros».
- [ ] **Un lector.** Con un dispositivo de lector, en una obra que falta: «Pedir que se añada». *Esperado:* llega como petición al administrador; su «Lo deseo» lo pone sin pedir nada.

## 6.6 · Series y «necesita»

- [ ] **Crear una saga.** Series → «Nueva serie», con orden; añade los tomos 1, 2 y 4. *Esperado:* el 3 sale como hueco «Sin apuntar» con «Apuntar aquí».
- [ ] **Reordenar.** «Reordenar», cambia números y «Hecho». *Esperado:* el orden se guarda y se sincroniza.
- [ ] **Desde la ficha.** «Añadir a una serie» en una obra. *Esperado:* elegir serie y número, o crear una nueva con o sin orden.
- [ ] **Una expansión.** En una expansión, edita y pon en «Necesita» el juego base, que no tienes. *Esperado:* aviso ámbar en la ficha de la expansión; en la del juego base, «Lo necesitan».

## 6.7 · Géneros y etiquetas

- [ ] **Géneros.** En un juego de mesa, elige «Estrategia» y «Colocación de trabajadores». *Esperado:* solo salen los géneros de juegos de mesa; en la ficha, como chips.
- [ ] **Etiquetas.** Escribe «para viajes» en dos obras. *Esperado:* la segunda vez la propone; tocar la etiqueta en la ficha filtra el catálogo.
- [ ] **Filtrar.** Filtra por un género y una etiqueta. *Esperado:* el catálogo se reduce y «Quitar filtros» lo deja como estaba.
- [ ] **Gestionar.** Ajustes de la biblioteca › Etiquetas: renombra «regalo» a una que ya existe. *Esperado:* avisa de que se unirán; tras «Unir», una sola etiqueta con las obras de las dos. Borrar una etiqueta la quita de las obras sin borrarlas.

## Sincronización

- [ ] **Todo viaja.** Con dos dispositivos, crea en uno una película, una serie con huecos, géneros, etiquetas y «lo deseo»; sincroniza. *Esperado:* el otro lo ve igual.
