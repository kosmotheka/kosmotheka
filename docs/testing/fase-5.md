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
