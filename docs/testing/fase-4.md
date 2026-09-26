# Pruebas manuales de la Fase 4

> Mismo formato que `hito-1.md`: pasos, resultado esperado, `[x]` al pasar. Las reglas de fusión y la migración tienen tests automáticos; aquí va lo que solo se ve con la app delante.

## Antes de empezar

La primera vez que abras esta versión, la app migra la base de datos (migración 9) y deja una copia al lado: `library.sqlite3.v8.bak` en la carpeta de datos. Si algo sale mal, cierra la app, borra `library.sqlite3` y renombra la copia.

## 4.5 · Varias bibliotecas

- [ ] **Tu biblioteca de siempre.** Tras migrar, el catálogo es el mismo y se llama «Mi biblioteca». *Esperado:* mismos libros, mismo historial.
- [ ] **Crear.** Selector de biblioteca (arriba a la izquierda en escritorio, menú ☰ en el móvil) → «Nueva biblioteca» → «Club de rol». *Esperado:* se abre vacía; el selector muestra las dos con su número de obras.
- [ ] **Nombre repetido.** Crea otra llamada «club de rol». *Esperado:* avisa de que ya existe y ofrece «Abrir Club de rol».
- [ ] **Cambiar.** Elige la otra biblioteca en el selector. *Esperado:* cambia el catálogo, el título y el contador; al cerrar y abrir la app sigue en la última elegida.
- [ ] **Renombrar.** Ajustes de la biblioteca → cambia el nombre → «Guardar». *Esperado:* el nombre nuevo aparece en el selector y en la barra lateral.
- [ ] **Mover un libro.** Abre un libro → menú ⋮ → «Mover a otra biblioteca» → elige destino. *Esperado:* desaparece de esta y aparece en la otra con sus ejemplares.
- [ ] **Buscar en todas.** Escribe algo que esté en las dos. *Esperado:* aparece «En esta · En todas»; con «En todas», los resultados salen agrupados por biblioteca y abrir uno de la otra cambia a ella.
- [ ] **¿Lo tengo? en todas.** Escanea o escribe el ISBN de un libro que esté en otra biblioteca. *Esperado:* «Lo tienes en 1 de tus 2 bibliotecas», con el nombre de la biblioteca en el libro.
- [ ] **Salir de una que solo está aquí.** Ajustes → «Salir de esta biblioteca». *Esperado:* avisa de que se perdería y solo ofrece «Ir a borrar».
- [ ] **Borrar con copia.** Ajustes → «Borrar biblioteca» → «Guardar copia de …» → elige dónde. *Esperado:* «Copia guardada» con la ruta; el botón pasa a «Borrar …». Al borrar, vuelves a otra biblioteca con un aviso.
- [ ] **Restaurar la copia.** Copias de seguridad → importa el `.zip` guardado. *Esperado:* vuelven los libros.
- [ ] **Borrar la última.** Borra todas. *Esperado:* la app crea una biblioteca vacía sin nombre («Mi biblioteca»); nunca te quedas sin ninguna.
- [ ] **Móvil.** Repite crear, cambiar y borrar en el móvil. *Esperado:* los diálogos salen como hojas desde abajo.

## 4.4 · Compartir por biblioteca

Con dos instancias (ver «Preparar dos instancias» en `fase-2.md`): **A** con «Casa» y «Club de rol», **B** vacía.

- [ ] **Elegir al emparejar.** En A → Compartir y dispositivos → marca solo «Club de rol» → «Mostrar código». En B → desmarca todo → pega el código → «Emparejar». *Esperado:* B recibe «Club de rol» con sus libros; «Casa» no aparece en B, y los libros de «Mi biblioteca» de B no llegan a A.
- [ ] **Los cambios van en los dos sentidos.** Añade un libro a «Club de rol» en B y sincroniza. *Esperado:* aparece en A.
- [ ] **Compartir otra después.** En A → dispositivos → «Recibe: Club de rol · Cambiar» → marca «Casa» → «Guardar». Sincroniza desde B. *Esperado:* «Casa» aparece en B.
- [ ] **Dejar de compartir.** En A → Ajustes de «Club de rol» → «Dejar de compartir» con B → confirma. Sincroniza desde B. *Esperado:* B avisa de que ya no tiene acceso a una biblioteca y «Club de rol» desaparece de B; A la conserva entera.
- [ ] **Desmarcar avisa.** En A → «Cambiar» → desmarca una. *Esperado:* antes de guardar, aviso de que desaparecerá del otro dispositivo.
- [ ] **Volver a dar acceso.** En A → vuelve a marcar «Club de rol» para B → sincroniza. *Esperado:* vuelve a B entera.
- [ ] **Salir no la trae de vuelta.** En B → Ajustes de «Club de rol» → «Salir» → sincroniza dos veces. *Esperado:* no vuelve a aparecer en B; en A, B deja de figurar en «Compartida con».

## 4.6 · Copias

- [ ] **Copia de una biblioteca.** Copias de seguridad → «Guardar copia de Casa». *Esperado:* el nombre propuesto es `casa-AAAA-MM-DD.zip`; el aviso cuenta libros, ejemplares y portadas.
- [ ] **Copia de todo el dispositivo.** Con dos bibliotecas o más → «Copia de todo el dispositivo». *Esperado:* el aviso dice cuántas bibliotecas lleva.
- [ ] **Restaurar en un dispositivo nuevo.** Con una segunda instancia vacía (`KOSMOTHEKA_DATA_DIR=/tmp/kosmotheka-c`) → «Restaurar copia» → la del dispositivo. *Esperado:* aparecen todas las bibliotecas con sus nombres, libros y portadas, y el resumen las lista una a una.
- [ ] **Restaurar lo que ya está.** En A, restaura la copia del dispositivo que acabas de hacer. *Esperado:* no se duplica nada; el resumen dice que cada biblioteca ya estaba.
- [ ] **Recuperar una borrada.** Borra una biblioteca guardando antes su copia, y restaura esa copia. *Esperado:* vuelve con su nombre y sus libros.
- [ ] **Nombre repetido.** Crea una biblioteca vacía llamada «Club» y restaura la copia de otra «Club». *Esperado:* la restaurada se llama «Club (2)».
- [ ] **CSV de la activa.** El apartado de hoja de cálculo lleva el nombre de la biblioteca activa, y el CSV exportado solo tiene sus libros.

