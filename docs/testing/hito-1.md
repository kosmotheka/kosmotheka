# Pruebas manuales del hito 1

> Lo que los tests automáticos no cubren: que la interfaz se vea y funcione. Cada prueba tiene pasos y el resultado esperado. Marca `[x]` al pasarla y anota debajo lo que falle.
>
> **Preparación:** `cd app && npm run tauri dev` (dentro del contenedor, ver `docs/development/setup.md`). Para empezar desde cero, cierra la app y borra `~/.local/share/com.tersoal.library/`.
>
> **ISBN de prueba:**
> - `978-0-306-40615-7` (y su ISBN-10 `0-306-40615-2`): ISBN válido que no corresponde a un libro real.
> - `9780306406158`: dígito de control erróneo.
> - `9788401352188`: *El nombre del viento* (Plaza & Janés), para las búsquedas online.

## 1.1 · Alta manual y listado

- [ ] **Catálogo vacío.** Sin libros, se ve el mensaje «El catálogo está vacío» con un botón para añadir.
- [ ] **Alta completa.** «Añadir libro» → título, subtítulo, dos autores (con «Añadir autor»), editorial, año, ISBN `978-0-306-40615-7`, idioma `es`, propietario, formato «Tapa dura», 1 unidad, fecha y ubicación → Guardar. *Esperado:* el diálogo se cierra, aparece el aviso «… se ha añadido al catálogo» y el libro sale en la tabla con sus autores en orden, el año, el formato y el propietario.
- [ ] **ISBN repetido.** Nuevo alta con el ISBN `0-306-40615-2` y formato «Ebook». *Esperado:* aviso «Ya teníais esta edición…»; la fila muestra 2 ejemplares y los dos formatos, sin fila nueva.
- [ ] **Errores a la vez.** Título vacío, año `0`, ISBN `9780306406158`, unidades `0` → Guardar. *Esperado:* no se cierra; aparece un error bajo cada uno de esos cuatro campos.
- [ ] **Autocompletar propietario.** En un alta nueva, el campo propietario sugiere los propietarios ya usados.
- [ ] **Idioma.** Los botones EN/ES cambian todos los textos, incluidos los formatos y los mensajes de error.

## 1.2 · Registro de operaciones e historial

- [ ] **Historial de un alta.** Abre un libro nuevo (clic en su título) → pestaña «Historial». *Esperado:* «Libro añadido» con fecha y hora.
- [ ] **Historial con ejemplar añadido.** En el libro del ISBN repetido: «Libro añadido» y después «Ejemplar añadido».
- [ ] **Libros anteriores al registro.** Un libro dado de alta antes del hito 1.2 muestra el aviso de que no tiene cambios registrados.

## 1.2b · Editar y borrar

- [ ] **Abrir la ficha.** Clic en el título (o en la fila). *Esperado:* ficha con las pestañas «Ficha», «N ejemplares» e «Historial».
- [ ] **Editar la ficha.** «Editar» → cambia el año y el orden de los autores, quita uno y añade otro → Guardar. *Esperado:* la ficha y la tabla muestran los cambios; el historial añade «Ficha editada».
- [ ] **Guardar sin cambios.** «Editar» → Guardar sin tocar nada. *Esperado:* no aparece ninguna entrada nueva en el historial.
- [ ] **ISBN de otro libro.** Edita un libro y pon el ISBN de otro. *Esperado:* error «Otro libro del catálogo ya tiene este ISBN.» y no se guarda.
- [ ] **Cancelar la edición.** «Editar» → cambia algo → Cancelar. *Esperado:* se vuelve a la vista sin cambios.
- [ ] **Añadir un ejemplar.** Pestaña de ejemplares → «Añadir ejemplar» → otro propietario y formato → Guardar. *Esperado:* aparece en la lista; el contador de la pestaña y la tabla se actualizan.
- [ ] **Editar un ejemplar.** Lápiz en un ejemplar → cambia la ubicación → Guardar. *Esperado:* se ve la nueva ubicación; el historial añade «Ejemplar editado».
- [ ] **Borrar un ejemplar.** Papelera → aparece una confirmación → Borrar. *Esperado:* desaparece; si era el último, se ve «Sin ejemplares…» y el libro sigue en la tabla con 0 ejemplares.
- [ ] **Borrar un libro.** Pestaña «Ficha» → «Borrar libro» → confirmación → Borrar. *Esperado:* la ficha se cierra y el libro desaparece de la tabla. Si vuelves a darlo de alta con el mismo ISBN, se crea como nuevo.
- [ ] **Cancelar un borrado.** En cualquiera de las dos confirmaciones, «Cancelar» no borra nada.

## 1.3 · Búsqueda por ISBN, portadas y enlaces

> Necesita conexión a internet. Google Books sin clave suele dar «cuota agotada»: lo normal es que los datos vengan de Open Library.
> ISBN útiles: `9780140328721` (*Fantastic Mr. Fox*, está en Open Library con portada) y `9788401352188` (*El nombre del viento*, no está en Open Library).

- [ ] **Buscar con el botón.** «Añadir libro» → el cursor empieza en el ISBN → escribe `9780140328721` → «Buscar». *Esperado:* spinner; después se rellenan título, autor (Roald Dahl), editorial (Puffin), año (1988) e idioma (`en`), aparece la portada a la izquierda del título y el mensaje «Encontrado en Open Library…». También aparece el aviso de cuota de Google.
- [ ] **Buscar con Intro.** Borra el formulario, escribe el ISBN y pulsa Intro. *Esperado:* busca, no guarda.
- [ ] **Guardar con portada.** Tras la búsqueda, pon un propietario → Guardar. *Esperado:* en la tabla sale la miniatura de la portada; en la ficha, la portada grande.
- [ ] **Enlaces.** En la ficha de ese libro, el botón «Open Library» abre su página en el navegador.
- [ ] **No encontrado.** Busca `9788401352188`. *Esperado:* «No está en los catálogos online…»; el formulario no se borra.
- [ ] **Ya en el catálogo.** Busca un ISBN que ya tengas. *Esperado:* aviso en naranja de que la edición ya está y de que se añadirá otro ejemplar.
- [ ] **ISBN inválido.** Busca `12345`. *Esperado:* «Un ISBN tiene 10 o 13 dígitos».
- [ ] **Sin conexión.** Desconecta la red y busca. *Esperado:* avisos de que no se pudo consultar cada servicio; nada se rompe y puedes rellenar a mano.
- [ ] **Buscar al editar.** En la ficha de un libro sin portada → «Editar» → pon su ISBN → Buscar → Guardar. *Esperado:* rellena los datos, guarda y aparece la portada. No sale el aviso de «ya en el catálogo» para el propio libro.
- [ ] **Ajustes.** Botón del engranaje → escribe una clave de Google Books → Guardar → vuelve a abrir Ajustes. *Esperado:* la clave sigue ahí (oculta). Con una clave válida, las búsquedas dejan de mostrar el aviso de cuota y el mensaje dice «Encontrado en Google Books, Open Library».
- [ ] **Verificar Google Books con clave real.** Con clave, busca `9788401352188` y comprueba que llegan título, autor y portada. *(Valida que la lectura de respuestas de Google, hecha sin una respuesta real, es correcta.)*

## 1.4 · Alta rápida (modo ráfaga)

> ISBN comprobados en Open Library el 25/09/2026: `9780140328721` (*Fantastic Mr. Fox*), `9780141346809` (*Percy Jackson and the Lightning Thief*), `9780747532699` (*Harry Potter and the Philosopher's Stone*), `9780261103573` (*The Fellowship of the Ring*), `9780553293357` (*Foundation*) y `9780345339683` (*The Hobbit*).

- [ ] **Abrir.** Botón «Alta rápida». *Esperado:* diálogo con propietario, formato, ubicación y un campo de ISBN grande con el cursor dentro.
- [ ] **Ráfaga.** Pon un propietario y una ubicación → escribe un ISBN + Intro → sin esperar, otro ISBN + Intro → y un tercero. *Esperado:* el campo se vacía al instante cada vez; las entradas aparecen arriba en cola («En cola…» → «Buscando…» → «Añadido») con título y portada; la tabla de detrás se actualiza sola.
- [ ] **Ya en el catálogo.** Escanea un ISBN que ya tengas. *Esperado:* «Ya estaba en el catálogo: no se añade» y el botón «Añadir otro ejemplar igualmente»; al pulsarlo pasa a «Ejemplar añadido» y en la tabla sube el número de ejemplares.
- [ ] **No encontrado.** Escanea `9788401352188`. *Esperado:* «No encontrado online» y un campo de título; escribe el título → Guardar → «Añadido»; el libro sale en la tabla con ese ISBN.
- [ ] **ISBN inválido.** Escribe `123` + Intro. *Esperado:* «ISBN no válido: Un ISBN tiene 10 o 13 dígitos».
- [ ] **Resumen.** La línea de resumen cuenta bien los añadidos, los que ya estaban y los pendientes.
- [ ] **Datos de ejemplar.** Abre la ficha de un libro añadido en ráfaga → pestaña de ejemplares. *Esperado:* el propietario, el formato y la ubicación elegidos.
- [ ] **Cambio a mitad de sesión.** Cambia el propietario y escanea otro. *Esperado:* el nuevo usa el propietario nuevo; los anteriores conservan el suyo.
- [ ] **Lector USB (si tienes uno).** Escanea un código de barras con el campo enfocado. *Esperado:* igual que teclear el ISBN y pulsar Intro.

## 1.5 · Buscar, filtrar y autocompletar

> Conviene tener varios libros con distintos formatos, propietarios y ubicaciones (el alta rápida ayuda).

- [ ] **Búsqueda sin acentos.** Escribe `senor` en el buscador. *Esperado:* aparece «El señor de los anillos…» sin pulsar nada más (medio segundo de espera como mucho) y la cabecera dice «1 resultado».
- [ ] **Prefijos y varias palabras.** `tolk anill` encuentra el libro; `tolkien dune` no encuentra nada, porque todas las palabras deben coincidir.
- [ ] **Por editorial y por ISBN.** Busca por el nombre de una editorial; después por un ISBN con guiones y en formato de 10 dígitos. *Esperado:* los encuentra.
- [ ] **Caracteres raros.** Escribe `"` o `*` o `(`. *Esperado:* no aparece ningún error.
- [ ] **Filtros.** Filtra por formato, propietario y ubicación, solos y combinados. Combinados significa «un mismo ejemplar que cumpla todo». *Esperado:* la lista se ajusta; «Quitar filtros» lo restablece.
- [ ] **Sin resultados.** Busca algo inexistente. *Esperado:* «Ningún libro coincide…» con botón para quitar filtros (no el mensaje de catálogo vacío).
- [ ] **Autocompletar autores y editoriales.** En «Añadir libro», al escribir en un autor o en la editorial aparecen sugerencias con los ya existentes.
- [ ] **Autocompletar propietario y ubicación.** En el ejemplar (alta, ficha y alta rápida) se sugieren propietarios y ubicaciones ya usados.
- [ ] **Editorial compartida.** Da de alta dos libros con `Minotauro` y `minotauro`. *Esperado:* los dos muestran «Minotauro» (una única editorial).
- [ ] **El índice sigue los cambios.** Edita el título de un libro y búscalo por el título nuevo (lo encuentra) y por el viejo (no lo encuentra). Borra un libro y comprueba que ya no sale en la búsqueda.
- [ ] **Migración de datos.** Los libros que tenías antes de este hito conservan su editorial (ahora como catálogo) y se encuentran en el buscador.

