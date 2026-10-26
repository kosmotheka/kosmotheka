---
title: Novedades de cada versión
description: "Lo que ha cambiado en cada versión de Kosmotheka, de la más nueva a la más antigua."
sidebar:
  order: 21
---

La app te enseña estas novedades al actualizarse, una sola vez. Si quieres volver a verlas, abre **Ajustes › Acerca de › Novedades**.

Cuando una novedad cambia lo que se envía desde tu móvil u ordenador a internet, la app te enseña a la vez un **interruptor** para que decidas tú si la quieres. Aquí lo indicamos con la palabra «interruptor».

Las líneas de cada versión son el texto que enseña la app, tal cual. Si una palabra no te suena: [La nube](../the-cloud/) explica la nube, el servidor y el segundo factor (el código extra al entrar, de la «app de autenticación»); [Compartir en casa](../share-at-home/), qué es emparejar; y el ISBN es el número que traen los libros junto al código de barras.

## Versión 0.7.0

- Fotos: en la ficha, toca la portada para poner una tuya, con una foto (en el móvil) o una imagen, o para buscarla otra vez en los catálogos después de añadir el ISBN o corregir el título. La tuya se guarda reducida y sin los datos de la foto, y llega a la nube con las demás portadas. Y para la primera carga, en el móvil, «Foto de una balda» en «Añadir» lee los lomos en el propio móvil, busca cada título en los catálogos y te deja marcar qué añadir de una vez. No lo lee todo, pero ahorra mucho escribir.
- Las «Series» pasan a llamarse «Sagas y colecciones», para no confundirlas con las series de televisión. Están en el menú, con lo de la biblioteca, y también se llega desde el Catálogo y «Me falta». La pantalla explica para qué sirven y enseña cuánto tienes de cada una, la ficha de una obra dice «2 de 6» en lugar de «Nº 2», meter una obra en un número ocupado corre las siguientes un puesto, y «+ Lo tengo» en una obra que te falta abre directamente el ejemplar para guardarlo. En el móvil, el menú se agrupa como en el ordenador y enseña tus tres últimas bibliotecas, con «Ver todas».
- Emparejar sin cámara es mucho más fácil: debajo del QR sale un número de 6 cifras, y en el otro dispositivo «Escribir el número» enseña los de tu wifi con el emparejamiento abierto; eliges el tuyo y escribes el número. Si el otro aún no tiene la app, «¿El otro dispositivo aún no tiene la app?» lleva a la descarga. Y «Invitar» también está en «Compartir y dispositivos».
- Si un catálogo no responde o se le ha acabado el cupo, el aviso se queda junto a los resultados, también en «¿Lo tengo?», y te lleva a Proveedores de datos cuando una clave lo arregla. Y al filtrar por género o etiqueta, lo elegido ya no sale dos veces en la barra.
- En el ordenador, la barra de arriba ya no se parte ni se sale en ventanas estrechas: el estado de sincronización ocupa una línea («hace 3 h») y «¿Lo tengo?» y «Alta rápida» se quedan en su icono. Los ajustes y las novedades son más anchos, la bienvenida y «Búsquedas a través de tu servidor» tienen el botón junto al texto, y «Personas y permisos» y «Compartir y dispositivos» ocupan todo el ancho.

## Versión 0.6.0

- En el móvil, si el código de barras de un libro es una etiqueta de la tienda, pulsa «Foto del número» en el escáner: la app lee el ISBN impreso en la foto, en el propio móvil, sin enviarla a ningún sitio. En «Añadir», «Escribir el número» saca el teclado numérico y ya no busca títulos mientras lo escribes.
- En «Personas y permisos», cada dispositivo dice cuándo hizo su último cambio. Si has reinstalado la app, te ofrece quitar de una vez las instalaciones anteriores, que se quedaban en tu lista para siempre.
- Al entrar con tu cuenta, una pantalla te cuenta qué catálogos consulta tu servidor por ti, qué se envía a cada uno y te deja apagar el que quieras. Vuelve a salir si tu servidor añade uno nuevo.
- La Ayuda explica también la nube y las personas, permisos y peticiones, y cada tema lleva a su página de la guía completa.
- En «Personas y permisos», el servidor de la nube ya no aparece como un dispositivo sin persona que se pueda asignar, se ve cuál es este dispositivo, y el aviso de administrador único no repite la lista entera cuando tienes muchos.

## Versión 0.5.0

- Las portadas pasan a los otros dispositivos al sincronizar, por la wifi de casa o por la nube: verás la misma portada en todos, aunque la obra se añadiera en otro.
- Las portadas ocupan mucho menos: se guardan más pequeñas y en un formato más ligero, también las que ya tenías. Se ven igual de bien.
- Al emparejar dos dispositivos que ya tenían bibliotecas, decir de quién es el otro ya no da un error de permisos ni lo deja sin dueño.
- Emparejar con el ordenador es más fácil: el código QR sale más grande y, en ordenadores con Docker o máquinas virtuales, mucho más sencillo, así que el móvil lo lee a la primera.

## Versión 0.4.0

- Con cuenta, al escanear un juego de mesa la app pregunta a GameUPC, a través del servidor de tu cuenta, qué juego es: ya no hace falta clave propia. Si no está seguro, eliges el tuyo entre sus propuestas o lo buscas por su nombre, y al guardarlo le decimos cuál era para que aprenda; le llega el código y el juego, con un identificador de tu cuenta que no dice quién eres. Puedes apagarlo en Proveedores de datos. *(Con interruptor.)*

## Versión 0.3.1

- Cuando una nube no es la oficial de Kosmotheka, la app lo dice: al escribir otra dirección de servidor, en la cuenta, en «Compartir y dispositivos», en la biblioteca, al recibir por QR una biblioteca que vive en otra nube y al abrir una invitación de otro servidor, siempre con quién la lleva.

## Versión 0.3.0

- «Salir de esta biblioteca» se llama ahora «Quitar de este dispositivo»: la biblioteca desaparece solo de ese dispositivo, también si está en la nube de tu cuenta (la app comprueba en ese momento que la nube la tiene), y después puedes volver a bajarla desde «Compartir y dispositivos». Nueva: «Salir de la biblioteca», con la que dejas de tener acceso en todos tus dispositivos y en tu cuenta; tus libros siguen en la biblioteca, a tu nombre.
- En el ordenador, el botón de arriba se llama «¿Lo tengo?» y ya no dice «Escanear», porque el ordenador no escanea con cámara: ahí se teclea o se pega el número del libro.
- «Borrar biblioteca» solo aparece a quien administra la biblioteca, y la lista de series y cada serie cuentan igual lo que tienes, huecos incluidos («tienes 3 de 6» en los dos sitios).
- La Ayuda enlaza con la guía completa, en docs.kosmotheka.com. Las portadas de cualquier proporción ocupan la misma caja en las listas, y elegir una biblioteca en el menú te saca de la Ayuda y te lleva a su lista.
- Las formas de quitar un dispositivo tienen nombres más claros («Dejar de emparejar», «Retirar el acceso de este dispositivo», «Sacar de mi cuenta» y «Dejar este dispositivo como nuevo»), y la app avisa antes de crear la cuenta de que la nube es, por ahora, por invitación.

## Versión 0.2.1

- La primera vez, la app te pide tu nombre: tu biblioteca se crea a tu nombre, como administrador o administradora, y lo cambias luego en Ajustes. Al emparejar un dispositivo hay que decir de quién es y con qué rol («editor» por defecto).
- Antes de usar esta versión hay que borrar los datos de prueba: en cada dispositivo, «Dejar este dispositivo a otra persona» en Ajustes; y en la nube, borrar la cuenta y registrarse otra vez. Además, restaurar una copia en un móvil nuevo no une la biblioteca con la original: queda como una biblioteca nueva con los mismos libros.
- Cambiar o restablecer la contraseña y borrar la cuenta piden el código de la app de autenticación, si la cuenta lo tiene.
- Al añadir, un ISBN con guiones o de 10 cifras se busca como ISBN; si su dígito de control no cuadra, la app lo dice en vez de preguntar qué es, y si ningún servicio lo conoce te deja añadir el libro a mano con el número ya puesto.
- Con cuenta, los juegos de mesa se buscan por título en BoardGameGeek, y al buscar libros salen también los de rol de RPGGeek. Lo que escribes va al servidor de tu cuenta, que lo consulta; puedes apagarlo en Proveedores de datos. *(Con interruptor.)*

## Versión 0.2.0

- Llega la nube, opcional y por invitación: con una cuenta (con correo o con Google) tienes tu biblioteca en todos tus dispositivos sin compartir wifi. Sin cuenta, todo sigue igual.
- Puedes invitar a otras personas a una biblioteca, por correo o con un enlace.
- Con cuenta, buscar un libro por su ISBN falla menos.
- Puedes quitar a una persona de una biblioteca y devolverle el acceso.

## Más

- Qué sale de tu dispositivo y qué no: [Qué sale de tu dispositivo](../what-leaves-your-device/) y [De dónde salen los datos](../data-sources/).
- Si algo no sale como esperabas tras actualizar: [Mensajes de error y qué hacer](../error-messages/).
