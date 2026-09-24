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
