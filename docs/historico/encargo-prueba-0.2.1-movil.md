# Encargo · Dos cosas de la prueba de la 0.2.1 en el móvil

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push, sin cambiar de rama. El maintainer probó la 0.2.1 de Google Play contra producción: BoardGameGeek y RPGGeek funcionan. Salieron dos cosas. Toca `app/`.

## 1. Las portadas de distinta proporción descuadran la lista

Un juego como «Catan» trae una imagen más ancha que la de un libro, y la lista parece descuadrada. **Todas las portadas ocupan la misma caja**, centradas dentro de ella, sin recortarse ni deformarse (`object-contain` y centrado). El texto de cada fila empieza siempre a la misma altura.

Mira dónde pasa: la lista de la biblioteca, los resultados de «Añadir» y cualquier otro sitio con miniaturas (series, ficha). Usa un solo componente o una sola clase para la miniatura, si no la hay ya. Compruébalo con `?mock`, metiendo una portada ancha y otra alta en el mock, en móvil (390×844) y en escritorio.

## 2. Desde «Ayuda» no se vuelve a la biblioteca

En el móvil: menú, «Ayuda» (bien), otra vez el menú y «Mi biblioteca» (o el nombre de la biblioteca). **No navega a la lista: no hay forma de salir de la Ayuda.**

- Reprodúcelo en `?mock` a tamaño de móvil, y escribe un test si hay dónde.
- Mira cómo se abre la Ayuda: si es una página o un panel, y cómo sabe la app qué pantalla enseñar (`lib/navigation`, `App.tsx`). Lo esperado: elegir una biblioteca en el menú, aunque sea la que ya está activa, lleva a su lista y cierra lo que tape.
- Comprueba también el botón atrás de Android desde la Ayuda.
- Revisa si «Acerca de», «Series», «Peticiones» u otras entradas del menú tienen el mismo problema, y arréglalo igual.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`; Rust si lo tocas. Las dos cosas son arreglos de la 0.2.1. Si caben, una línea en `whats-new.json` bajo un `next` nuevo; si no, nada. Capturas del antes y el después del punto 1.
