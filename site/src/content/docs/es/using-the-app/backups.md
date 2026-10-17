---
title: Copias de seguridad, exportar e importar
description: Cómo guardar una copia de tu biblioteca, recuperarla y pasar tu lista a una hoja de cálculo o desde ella.
sidebar:
  order: 10
---

Tus bibliotecas viven en tu móvil u ordenador. Si lo pierdes, se rompe o lo borras sin querer, solo recuperas lo que tengas en una copia de seguridad. Conviene hacerlas de vez en cuando.

Para empezar, abre el menú (en el móvil, las tres rayitas de arriba a la izquierda) y entra en «Copias de seguridad».

![El diálogo «Copias de seguridad».](./backups-dialog.webp)

## Guardar una copia de seguridad

Una copia de seguridad es un archivo con tus libros y sus portadas.

1. Entra en «Copias de seguridad».
2. Deja activado «Proteger con contraseña». Ya viene activado.
3. Escribe una contraseña de al menos 8 caracteres y repítela.
4. Pulsa «Guardar copia de» y el nombre de tu biblioteca.
5. Elige dónde guardar el archivo: una carpeta, una memoria USB o tu Google Drive, por ejemplo.

Al terminar, la app dice cuántos libros, ejemplares y portadas ha guardado.

**Apunta la contraseña.** Sin ella nadie puede abrir la copia, y la app no la guarda en ningún sitio. Guárdala en un papel o en el programa de contraseñas que uses.

Puedes quitar la contraseña con el botón de «Proteger con contraseña». La app avisa de que cualquiera que encuentre el archivo podrá verlo: los nombres, lo que tenéis y quién tiene qué.

## Guardar todas tus bibliotecas de una vez

Si tienes más de una biblioteca, el botón «Copia de todo el dispositivo» las guarda todas en un solo archivo.

## Recuperar una copia de seguridad

1. Entra en «Copias de seguridad» y pulsa «Restaurar copia».
2. Elige el archivo.
3. Si tiene contraseña, escríbela y pulsa «Desbloquear y restaurar». Si te equivocas, no se recupera nada y puedes volver a intentarlo.

Cada biblioteca del archivo vuelve con su nombre. Las que ya tienes en tu dispositivo con libros no se tocan.

**Una cosa que conviene saber.**

- Si recuperas la copia en el mismo dispositivo que la hizo, la biblioteca vuelve tal cual.
- Si la recuperas en otro dispositivo, aparece como una biblioteca nueva con los mismos libros, y **no se junta con la original**.

Para tener en un dispositivo nuevo la biblioteca original, pásala desde otro dispositivo que la tenga (mira [Compartir en casa](../share-at-home/)) o bájala de tu cuenta, si tienes una.

Si la app dice que la copia se hizo con otra versión de la app, actualiza la app y vuelve a intentarlo.

## Tu lista en una hoja de cálculo

En la misma pantalla, la parte «Hoja de cálculo» sirve para pasar tu lista a Excel, LibreOffice u otro programa parecido, o para traer una lista que ya tengas hecha.

### Exportar

Pulsa «Exportar CSV» y elige dónde guardar. Un CSV es un archivo que Excel y otros programas pueden abrir, con una línea por cada ejemplar (cada uno que tienes de algo, en papel o en digital). La app dice cuántas líneas ha escrito.

El CSV **no tiene contraseña**: cualquiera que lo encuentre puede leerlo. Tampoco lleva todo: no guarda el tipo (libro, película…), ni las portadas, ni las series, ni lo que marcaste como leído, ni las estrellas.

### Importar

Pulsa «Importar CSV» y elige el archivo. La primera línea del archivo tiene que ser los nombres de las columnas. **Esos nombres van en inglés**, escritos tal cual:

| Columna | Qué lleva |
|---|---|
| `title` | El título. **Es lo único obligatorio.** |
| `subtitle` | El subtítulo |
| `authors` | Quién lo ha escrito. Si son varias personas, sepáralas con punto y coma: `Margaret Weis; Tracy Hickman` |
| `publisher` | La editorial |
| `published_year` | El año, con cifras |
| `language` | El idioma, con su código de dos letras: `es` para español, `en` para inglés… |
| `isbn13` | El ISBN, el número que lleva cada libro junto al código de barras |
| `format` | El formato, con una de las palabras de la tabla de abajo |
| `owner` | De quién es |
| `quantity` | Cuántas unidades. Si no pones nada, una |
| `location` | Dónde está |
| `acquired_on` | La fecha de compra, como `2026-10-16` |

En la columna `format` hay que usar estas palabras, también en inglés:

| En la app | En el CSV |
|---|---|
| Tapa dura | `hardcover` |
| Tapa blanda | `paperback` |
| Bolsillo | `pocket` |
| Ebook | `ebook` |
| PDF | `pdf` |
| Audiolibro | `audiobook` |
| Otro, o sin formato | `other`, o déjalo vacío |

Qué pasa al importar:

- Todo entra como libro. Después puedes cambiar el tipo de cada uno desde su ficha.
- Si el ISBN de una línea ya está en tu biblioteca, no se repite el libro: se le suma un ejemplar.
- Las líneas con errores se saltan y el resto se importa. Al terminar, la app dice cuántos libros nuevos hay, a cuántos libros que ya tenías se les ha sumado un ejemplar y cuántas líneas se han rechazado, con el número de cada una.
- Si hay una columna que la app no conoce, la ignora.

Puedes probar primero con una biblioteca nueva, para ver cómo queda antes de mezclarlo con la tuya.
