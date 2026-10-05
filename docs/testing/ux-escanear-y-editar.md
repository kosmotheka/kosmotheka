# UX: escanear un libro que no está y editar una ficha

Prueba del maintainer en el móvil (versión 0.1.4 de prueba interna, 05/10/2026). Libro: *El temor de un hombre sabio*, que ningún proveedor encuentra por ISBN.

## Lo que pasó

1. **Escanear un ISBN desconocido.** «¿Lo tengo?» dice «No lo tienes» y ofrece un campo «Título para acordarte» con tres botones: «Apuntar como me falta» (destacado), «Lo he comprado» y «Escanear otro».
   - Esperaba un botón **Guardar**. No se entiende que «Apuntar como me falta» es lo que guarda el título.
   - **Intro en el teclado no hace nada:** el campo de título no está en un `<form>` (`app/src/pages/check-page.tsx`).
2. **«Lo he comprado»** abre el formulario de alta con el ISBN y el título. La búsqueda por ISBN no encuentra nada (ningún proveedor tiene ese ISBN). El ISBN sí venía relleno.
3. **Guardar** vuelve a «¿Lo tengo?», con el aviso de guardado, pero la pantalla **sigue diciendo «No lo tienes»** para ese ISBN. La comprobación no se repite después de guardar.
4. **Añadir buscando por título** encuentra el libro en un proveedor en inglés (no lo quería) y en uno en español que **solo trae el título**.
5. **Editar desde la ficha:** el ISBN sí estaba guardado. Al buscar, Google Books dice **cuota agotada** (la clave compartida).
6. **El formulario de edición se abre debajo de la ficha.** El botón Guardar queda abajo del todo; al subir, el dedo vuelve a pulsar el lápiz, y el formulario se cierra sin guardar.

## Propuestas (por hablar)

| # | Problema | Propuesta |
|---|---|---|
| 1 | No se sabe cómo guardar lo escaneado | Un único botón principal claro («Guardar en Me falta»), e Intro en el título hace lo mismo |
| 2 | Tres botones al mismo nivel | Principal: guardar en «Me falta»; secundario: «Lo he comprado»; «Escanear otro» como enlace o en la barra |
| 3 | Tras guardar sigue «No lo tienes» | Volver a comprobar el ISBN al guardar, o pasar directamente a escanear otro |
| 4 | Resultados en otro idioma o sin datos | Ordenar por el idioma de la app y por cantidad de datos; agrupar resultados de la misma obra |
| 5 | Cuota de Google Books agotada | Ya hay clave propia en Ajustes, pero nadie la va a poner. El maintainer prefiere buscar a través del servidor, pidiendo permiso y explicando que es solo para eso; acepta, si no hay otra, una clave del proyecto en la app. Ojo: el plan gratuito de Google Books se agota pronto con muchos usuarios, así que no debe ser la fuente principal |
| 6 | Editar debajo de la ficha | En el móvil, editar es una **pantalla propia**, con Guardar arriba donde estaba el lápiz, y Atrás pregunta si hay cambios sin guardar |

## Hecho (05/10/2026)

- Intro en el título guarda en «Me falta», y el botón se llama «Guardar en Me falta» (propuesta 1).
- Después de «Lo he comprado» y guardar, la pantalla vuelve a comprobar el ISBN (propuesta 3).

- Una obra en «Me falta» ya no cuenta como «Lo tienes»: `check_isbn` separa las que tienen ejemplar de las que faltan, y la pantalla dice «No lo tienes · Ya está en Me falta». Versión 0.1.5.

## Fuentes de libros en español, por probar

Propuestas por el maintainer. Antes de incluir ninguna hay que comprobar si tiene una API o datos abiertos (no solo una web), sus condiciones de uso y cuántos libros españoles encuentra de verdad, con una lista de ISBN de prueba (entre ellos *El temor de un hombre sabio*).

| Fuente | Qué es | Por comprobar |
|---|---|---|
| Base de datos de libros editados en España (Ministerio de Cultura, agencia del ISBN) | Todo lo editado en España, por título o ISBN | Si hay API o descarga; si no, leer la web es frágil |
| TodosTusLibros (CEGAL, librerías) | Libros editados en España y su disponibilidad | API y condiciones |
| BiblioScan | Herramienta para librerías de segunda mano que venden en Amazon (escaneo de baldas, precios de Keepa) | Tiene API (`/api-docs`), con clave `bsk_` por cuenta y créditos de pago. No encaja: cada persona necesitaría su cuenta de pago |
| Google Books | Ya lo usamos | La cuota (propuesta 5) |
| *Añadida:* Biblioteca Nacional de España (datos.bne.es) | Catálogo de la BNE como datos abiertos | Probar el acceso y la cobertura |

## Otras ideas de la misma búsqueda

- **LibraryThing** (app móvil): busca en Amazon y en más de 5000 bibliotecas. Es el listón de referencia, no una fuente: comprobar si sus servicios web siguen abiertos a terceros.
- **Buscar con una foto de la portada** (Google Lens, bookidentifier.com, apps de Play): ya está en la Fase 10. Google Lens no tiene API pública. Vías posibles: reconocer el texto en el móvil (OCR local, sin red) y buscar con él, y la IA opcional para los casos difíciles (D19, D20).

## Segunda prueba (05/10/2026)

- **El código escaneado no era un ISBN:** `2401114882038` empieza por 2, el prefijo de los códigos internos de tienda. La app lo aceptó como ISBN porque solo comprueba la longitud y el dígito de control. Propuesta: exigir el prefijo 978 o 979 y avisar «Este código no es un ISBN; busca el que empieza por 978».
- **La ficha no muestra el ISBN:** al abrirla no sale; en Editar sí está; tras guardar sin tocar nada aparece en la ficha. **Causa (comprobada en el escritorio):** no es un fallo de datos. La ficha se abre siempre en la pestaña «Ejemplares» y el ISBN está en «Datos»; al guardar desde Editar se queda en «Datos». Va con la propuesta 6 (editar en su propia pantalla): los datos principales, ISBN incluido, deberían verse sin cambiar de pestaña.

## Hecho después (05/10/2026)

- Un ISBN-13 tiene que empezar por 978 o 979 (`isbn_prefix`). Solo cambia la validación de lo que se escribe o escanea: el formato de las operaciones en `spec/` no cambia, así que los datos ya guardados siguen sincronizando.
- El escáner de «¿Lo tengo?» salta los códigos de tienda con un aviso y sigue con la cámara abierta, y tiene un botón «Escribir el número».
- La Biblioteca Nacional de España es el primer proveedor de libros.

## Prueba en el escritorio (05/10/2026)

- Buscar `9788499899619` en «Añadir»: el primer resultado es de la **BNE** (título, autoría, editorial, año), con la portada de Open Library. La primera búsqueda de la sesión dio «No se pudo consultar» para la BNE, Google Books y Open Library a la vez, y al repetir funcionó: parece un fallo puntual de red al arrancar, pero conviene vigilarlo.
- **La portada «se perdía» al guardar:** sí se descargaba y se guardaba, pero no se veía. El permiso para mostrar imágenes locales (`assetProtocol.scope` en `tauri.conf.json`) solo cubre la carpeta de datos por defecto, y la prueba usaba otra (`KOSMOTHEKA_DATA_DIR`). Solo pasaba con esa variable, en desarrollo. Arreglado: al arrancar se permite la carpeta de portadas real. Comprobado en el escritorio.
- Las portadas que tardan en llegar ya no dejan un hueco que parece definitivo: la caja late hasta que llega la imagen.
- El idioma salía como código («es») y se escribía a mano. Ahora el formulario tiene un **selector** con unos cuarenta idiomas por su nombre, en el idioma de la app (con `Intl.DisplayNames`, sin tabla propia), y la ficha muestra el nombre. Se sigue guardando el código ISO 639-1. Un código que no esté en la lista, o un texto libre de antes, se conserva y se puede elegir. Incluye el chino. Pendiente de probar en Android, donde el desplegable es una lista larga sin buscador.

## Editar en su propia pantalla (05/10/2026)

Hecho (propuesta 6): al pulsar el lápiz, la ficha se sustituye por el formulario, con «Guardar» arriba, donde estaba el lápiz (en el móvil, una X a la izquierda para cancelar). Al guardar o cancelar se vuelve a la ficha **en la misma pestaña** que estaba, así que ya no cambia de pestaña según cómo se llegue. Cancelar con cambios pregunta antes de descartarlos.

## Pendiente

- **Botón atrás de Android mientras se edita:** hoy cierra la ficha entera y se pierde lo escrito. Debería cancelar la edición (y preguntar si hay cambios), con una entrada propia en el historial, como el escáner.
- **Datos principales a la vista:** el ISBN y la editorial siguen en la pestaña «Datos». Valorar mostrarlos en la cabecera de la ficha.
- **Ordenar los resultados de búsqueda** por el idioma de la app y por cantidad de datos, y agrupar los de la misma obra (propuesta 4).
- **Open Library sin autoría:** pedir también la obra cuando la edición no trae autores.
- **Búsqueda a través del servidor** (D91), con límite por IP, y la **Library of Congress** para inglés (solo HTTP). Con la 7.4. Al añadirla, subir `WELCOME_VERSION` y añadirla a la bienvenida.
- **La primera búsqueda de la sesión** falló una vez en todas las fuentes a la vez; vigilar si se repite.
- **Selector de idioma en Android:** si la lista larga se hace pesada, cambiarlo por uno con buscador.
- **Leer el ISBN impreso con la cámara:** plugin libre de Tauri en su propio repositorio (Fase 10).
- **Validar el idioma en `spec/`:** hoy es texto libre en el formato de las operaciones; el formulario ya solo guarda códigos.
