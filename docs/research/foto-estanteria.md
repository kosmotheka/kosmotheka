# Foto de la estantería: primera prueba con fotos reales

Notas de la prueba del 12/10/2026 para la foto de balda de la Fase 10 (D19, vías 3 y 4). No hay nada desarrollado: es la medida que D19 pedía antes de decidir.

## Qué se probó

Seis fotos de las estanterías del mantenedor, hechas con el móvil sin preparar nada, leídas por un modelo de visión grande (Claude), que es la vía 4 de D19. Las fotos no están en el repositorio.

**El lector de texto del propio móvil (ML Kit, vía 3) no se ha probado todavía.** Es la prueba que falta.

## Resultado

| Foto | Qué sale | Lomos leídos |
|---|---|---|
| Pila de 12 libros de cerca, lomos en horizontal | Título y código de colección de cada uno, incluidos dos lomos finos | 12 de 12 |
| Tramo de balda de cerca, lomos en vertical, letra gótica y plástico con reflejos | Título y editorial | Unos 16 de 18 |
| Mueble entero (cuatro fotos, de frente y en ángulo, una con flash) | Solo los lomos gruesos de letra grande y las cajas de juegos | Menos de la mitad, a ojo |

En las fotos del mueble entero no se lee:
- las colecciones encuadernadas iguales, con el título en una etiqueta pequeña;
- lo que está en bolsas de plástico;
- los lomos finos (módulos, revistas, cómics grapados);
- lo que queda cortado por el borde de la foto.

Parte de lo que sí sale en esas fotos se reconoce más por el diseño del lomo que por leerlo. Ahí es donde un modelo puede inventar un título, y la persona no lo nota si no lo revisa.

## Lo que se saca

- **La foto es de un tramo, no del mueble.** Entre 15 y 20 lomos por foto. La pantalla tiene que pedirlo así.
- **Del lomo sale el título, y a veces la editorial y la autoría; nunca el ISBN.** Después hay que buscar cada título en los proveedores y elegir la edición. Es una carga asistida, con la cola de revisión de D19: foto, lista de títulos leídos, la persona confirma y elige candidato.
- **Encaja con elegir varios resultados de una búsqueda a la vez**, idea apuntada en `docs/plans/fase-7.md`.
- **Depende mucho de la colección.** La novela se encontrará casi toda en Google Books o la BNE. Los juegos de rol, módulos y revistas apenas están en esos catálogos: muchos se quedarían con el título leído y poco más.
- **Con un modelo grande, la foto sale del dispositivo** y cada foto cuesta dinero. Sigue siendo lo que dice D19: opcional, con clave propia o la del servidor (D18, D20).

## Lo que falta por probar

1. **Las mismas fotos con ML Kit en el móvil**, que es lo que se usaría sin enviar nada fuera. Se espera peor resultado con lomos en vertical y letra decorativa; hay que medirlo.
2. **Cuántos de los títulos leídos encuentran su ficha** en los proveedores que ya hay.
3. Fotos de cómics, películas y videojuegos.

## Ideas habladas el 12/10/2026, sin decidir

- **El lector de texto del móvil va dentro de la app**, no en una app de prueba aparte: publicar otra app en Google Play solo para probar no compensa. El módulo se desarrolla aparte para publicarlo como plugin libre de Tauri, igual que el del ISBN impreso.
- **El modelo de visión, con clave propia y una lista corta de proveedores.** No puede ser solo Claude, y tampoco cualquier integración. Qué proveedores entran es D20, que sigue abierta.
- **Qué modelo hace falta.** Si uno pequeño y barato lee lo mismo que uno grande, cambia el coste y qué clave hay que crear. Comparación en la sección siguiente.

## Modelo pequeño frente a modelo grande (12/10/2026)

Las mismas seis fotos, leídas por Claude Haiku con un encargo neutro: una línea por lomo y una marca de «leído», «dudoso» o «supuesto». No sabía qué había leído el modelo grande. Lo que el modelo grande leyó lo dio por bueno el mantenedor.

| Foto | Modelo grande | Haiku |
|---|---|---|
| Pila de 12, de cerca | 12 de 12 | 12 de 12, con los dos lomos finos marcados como dudosos |
| Tramo de balda, de cerca | Unos 16 de 18 | Los 13 libros bien, con su editorial; las revistas del borde, a medias |
| Mueble entero (cuatro fotos) | Menos de la mitad, sin inventar | Lee los lomos grandes, pero **inventa** el resto |

Lo que Haiku inventó en las fotos del mueble entero:
- títulos que no están (dos de Michael Crichton que no existen, dos de Stephen King que no son los de la balda);
- autores cambiados (una novela de John Verdon atribuida a Isabel Allende, dos de N. K. Jemisin metidas entre las de Sanderson);
- nombres de colección falsos para los tomos que no se leen;
- y contó 3 a 6 lomos sin leer por foto, cuando son varias decenas.

Casi todo lo inventado iba marcado como «dudoso», así que la marca sirve para avisar. No siempre: algún título mal leído iba como «leído».

**Lo que se saca.**
- **De cerca, el modelo pequeño lee lo mismo que el grande.** Como la foto tiene que ser de un tramo de todas formas, el modelo barato puede bastar.
- **De lejos, el pequeño es peor que no leer:** rellena con títulos verosímiles. La app no debe aceptar fotos de mueble entero, o debe trocearlas antes.
- **Pedir la marca de seguridad merece la pena**, y lo «dudoso» va a la cola de revisión sin preseleccionar.
- Son solo dos fotos de cerca. Antes de decidir hacen falta más: cómics, lomos finos, cajas, películas.
