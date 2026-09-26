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

Pendiente: se añadirá cuando se construya.
