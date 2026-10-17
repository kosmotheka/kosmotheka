---
title: "De dónde salen los datos: catálogos públicos, claves propias y cómo apagarlos"
description: Qué servicios de internet consulta la app cuando añades algo, qué les envía, cómo poner una clave propia y cómo apagar uno.
sidebar:
  order: 11
---

La app no lleva guardados los datos de los libros, las películas o los juegos. Cuando añades algo, los **busca por ti** en servicios de internet (catálogos públicos, como el de la Biblioteca Nacional de España). Esta página explica cuáles son, qué les llega y cómo elegir cuáles usar. Para el resumen de todo lo que sale de tu dispositivo, mira [Qué sale de tu dispositivo y qué no](../what-leaves-your-device/).

Si prefieres no usar ninguno, apágalos todos: todo se puede añadir a mano.

## Dónde se cambia

![La pantalla «Proveedores de datos», sin cuenta: para cada tipo, cuántos servicios se pueden usar.](./data-sources-dialog.webp)

Abre el menú, entra en «Ajustes» y pulsa «Proveedores de datos». A la izquierda eliges «Todas» o un tipo (libros, cómics, películas…), y ves los servicios que se usan para ese tipo. Cada tipo dice «N de M»: cuántos de sus servicios puede usar ahora. Si hay pocos, sale en naranja: buscará poco. «Otro» no busca en ningún sitio, se añade a mano.

Cada servicio tiene un **interruptor** para encenderlo o apagarlo, y una etiqueta que dice qué necesita:

- **«Sin clave»:** funciona ya, sin hacer nada. Vienen encendidos.
- **«Pide clave»:** necesita una clave propia que te da su web. Hasta que la pongas, no se usa.
- **«Pide cuenta»:** solo funciona a través del servidor de una cuenta de Kosmotheka (más abajo).
- **«Pide clave o cuenta»:** vale una clave tuya o una cuenta en un servidor que lo ofrezca.

## Qué servicios hay para cada tipo

| Tipo | Servicios | Qué necesitan |
|---|---|---|
| Libro | Biblioteca Nacional de España, Library of Congress, Open Library, Inventaire, Wikidata | Nada |
| Libro | Google Books (la más completa por ISBN) | Clave propia, o cuenta |
| Libro | RPGGeek (libros de rol, solo por título) | Cuenta |
| Cómic | Wikidata | Nada |
| Cómic | Comic Vine | Clave propia, o cuenta |
| Juego de mesa | Wikidata | Nada |
| Juego de mesa | GameUPC (por el código de barras de la caja) | Clave propia |
| Juego de mesa | BoardGameGeek | Cuenta |
| Película y serie | Wikidata | Nada |
| Película y serie | TMDB | Clave propia, o cuenta |
| Videojuego | Wikidata | Nada |
| Videojuego | RAWG | Clave propia, o cuenta |
| Música | MusicBrainz | Nada |

Las portadas se bajan del servicio que las tiene y se guardan en tu dispositivo.

Sin clave ni cuenta, los cómics, los juegos de mesa, las películas, las series y los videojuegos se buscan solo en Wikidata, una enciclopedia abierta de datos. Funciona, pero a veces faltan portadas o ediciones; por eso aparece el aviso.

## Poner una clave propia

Una clave es una contraseña gratuita que te da la web del servicio para dejarte usar sus datos.

1. En «Proveedores de datos», abre el servicio que pide clave.
2. Pulsa «Cómo conseguir una clave» y sigue los pasos de su web.
3. Pega la clave en «Pega aquí tu clave» y pulsa «Guardar».
4. Pulsa «Probar». La app te dice «La clave funciona.» o «Esta clave se ha rechazado.».

Con la clave puesta, la app le pregunta **directamente** al servicio y no pasa nada por ningún servidor de Kosmotheka («Usa tu propia clave»). Para quitarla, usa «Quitar la clave de…». En TMDB, la que hay que pegar es el «API Read Access Token», no la clave corta.

## Apagar un servicio

Pulsa el interruptor del servicio. Deja de recibir preguntas y no sale en los resultados. La clave que hubieras puesto se queda guardada, por si lo enciendes otra vez.

## Qué se envía a cada servicio

Solo **lo que buscas**: el título que escribes, o el número del código de barras (por ejemplo, el ISBN). Nunca tu lista de libros, tu nombre ni lo que tienes. Como cualquier página web, cada servicio ve desde qué dirección de internet le preguntas y que la pregunta viene de Kosmotheka, con su versión.

## Si tienes cuenta: algunos van a través del servidor

Con una cuenta, algunos servicios se pueden consultar a través del servidor de tu cuenta, que busca con su propia clave y por eso **no necesitas poner la tuya**. Son Google Books, TMDB, RAWG, Comic Vine, BoardGameGeek y RPGGeek, y solo si tu servidor los ofrece.

En ese caso la app envía a tu servidor lo que escribes (y, según el servicio, si es película o serie, o el idioma de la app) junto con tu sesión, que necesita para saber que tienes cuenta. El servicio ve la búsqueda como del servidor, no tuya. El servidor guarda la respuesta unos días para no repetir la pregunta, **sin guardar quién la pidió ni el texto que escribiste**.

Para que no pase por el servidor:

- pon tu propia clave, que tiene preferencia, o
- apaga el servicio. En Google Books hay además un interruptor propio, «Buscar en Google Books a través de la nube», que viene encendido.

BoardGameGeek y RPGGeek no admiten clave propia: sin cuenta, esos dos no se usan.

Los detalles (qué guarda el servidor y cuánto tiempo) están en la [política de privacidad](https://kosmotheka.com/es/privacidad.html).

## Créditos

Los datos de TMDB, RAWG, Comic Vine y BoardGameGeek tienen su aviso en la propia pantalla («Créditos»). Esta aplicación usa los datos de TMDB y de BoardGameGeek, pero no está respaldada ni relacionada con ellos.
