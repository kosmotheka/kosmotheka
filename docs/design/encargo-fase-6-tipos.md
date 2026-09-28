# Encargo de diseño: tipos de obra en la interfaz (Fase 6, hito 6.2)

> Primer encargo de la Fase 6 para Claude Design, 28/09/2026. Lo decidido está en D60 y en [plans/fase-6.md](../plans/fase-6.md). Escritorio y móvil, claro y oscuro.

## Contexto

Kosmotheka cataloga lo que tiene una familia o un grupo. Hasta ahora todo era libro. Desde esta fase hay ocho tipos cerrados (el usuario no crea tipos): Libro, Cómic, Juego de mesa, Película, Serie, Videojuego, Música y Otro. Ya existen pantallas que sirven de base: las de la Fase 3 (catálogo con chips de tipo, alta que empieza por buscar con «¿Qué es?» y «Añadir a mano») y las de la Fase 5.8 (marcas por persona, catálogo con todos los tipos). Hay que mantener ese lenguaje visual.

Lo común a todos los tipos: título, año, portada, notas, y sus ejemplares (formato, dónde está y de quién es).

| Tipo | Datos propios | Formatos del ejemplar |
|---|---|---|
| Libro | autoría, editorial, ISBN, idioma | tapa dura, tapa blanda, bolsillo, ebook, PDF, audiolibro |
| Cómic | guion, dibujo, editorial, número | tapa dura, tapa blanda, grapa, digital |
| Juego de mesa | diseño, editorial, jugadores (mín.–máx.), duración, edad mínima | caja, digital |
| Película | dirección, duración | DVD, Blu-ray, 4K, digital |
| Serie | temporadas | DVD, Blu-ray, digital |
| Videojuego | plataforma, estudio | físico (cartucho o disco), digital |
| Música | artista, sello | vinilo, CD, casete, digital |
| Otro | nada propio | otro |

Nada de sinopsis, reparto, páginas ni pistas. Guion, dibujo, diseño, dirección, estudio y artista son personas o grupos, como la autoría de un libro: autocompletado y puede haber varias. Lo que no es libro puede tener un código de barras (EAN/UPC) en lugar de ISBN. Lo digital no tiene estantería: su «dónde» es la plataforma («Kindle», «Steam», «Spotify»).

## Qué se pide

1. Elegir el tipo al añadir a mano y cambiarlo al editar, y qué pasa con los datos propios si se cambia (propuesta: avisar de que se perderán).
2. El formulario de alta y edición de cada tipo: un único formulario cuya parte específica cambia; «Otro» es el formulario sin parte específica.
3. La ficha de una obra de cada tipo.
4. El ejemplar: solo los formatos de su tipo; en los digitales, el «dónde» es la plataforma.
5. La portada con la forma de cada tipo, también sin imagen (marcador por tipo).
6. Catálogo: los chips de tipo y una lista mezclada.
7. Marcas: los textos de los cuatro estados para cada tipo («Jugando», «Viendo», «Escuchando»…).

Fuera: proveedores (6.3), lo que falta y lo que deseo (6.5), series (6.6), géneros y etiquetas (6.7).
