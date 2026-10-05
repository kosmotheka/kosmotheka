# Fuentes de libros en español

> Investigación del 05/10/2026, tras la [prueba en el móvil](../testing/ux-escanear-y-editar.md). Solo investigación: no cambia el ROADMAP; sirve para decidir si se adelanta. Amplía [proveedores.md](proveedores.md).

## Cómo se ha probado

Seis ISBN de editoriales españolas, consultados a mano con `curl` y un script de Python (sin clave en ningún caso):

| ISBN | Libro |
|---|---|
| 9788401339639 | *El temor de un hombre sabio*, Plaza & Janés, 2011 |
| 9788401352331 | *El temor de un hombre sabio*, Plaza & Janés, otra edición |
| 9788499899619 | *El temor de un hombre sabio*, Debolsillo |
| 9788497592208 | *Cien años de soledad*, Debolsillo |
| 9788490663196 | *Patria*, Tusquets |
| 9788433977656 | *El grito de la lechuza*, Anagrama |

## Resultados

| Fuente | Encontrados | Qué trae | Notas |
|---|---|---|---|
| **Biblioteca Nacional de España** (SRU de su catálogo Alma) | 5 de 6 | Título, **autoría**, editorial, año, páginas, ISBN | Sin clave, ~0,6 s por consulta. También busca **por título**. Devuelve también la edición electrónica («1 recurso en línea»), que hay que distinguir. Sin portadas |
| Open Library (ya integrado) | 6 de 6 | Título, editorial, año; portada en 4 | En 3 de 6 la **edición no trae autoría** (está en la obra, otra consulta) |
| Inventaire (ya integrado) | 5 de 6 | Título | Habría que ver cuánto más trae al pedir la obra |
| Google Books (ya integrado) | — | — | Sin clave, la cuota diaria compartida estaba **agotada** al probar |
| Base de datos del ISBN (Ministerio de Cultura) | — | — | Solo una web con formulario; no hay API. Leer la web es frágil |
| TodosTusLibros (CEGAL) | — | — | La búsqueda por ISBN lleva a la ficha web; no se ha visto API pública |
| BiblioScan | — | — | API de pago, por cuenta, para librerías que venden en Amazon. No encaja |

Consulta a la BNE (SRU 1.2, MARC21):

```
https://bne.alma.exlibrisgroup.com/view/sru/34BNE_INST?version=1.2&operation=searchRetrieve&recordSchema=marcxml&query=alma.isbn=9788401339639
```

Campos MARC útiles: 245 (título), 100 (autoría), 264/260 (editorial y año), 300 (páginas), 020 (ISBN). Para título: `query=alma.title="temor de un hombre sabio"`.

## Conclusiones provisionales

- **La BNE es la mejor candidata** para libros editados en España: datos completos, rápida y sin clave, así que funcionaría también sin nube. Falta comprobar sus **condiciones de uso** para un servicio así y si limita las peticiones.
- **Open Library ya encuentra estos libros**, pero sin autoría en la mitad. Pedir también la obra mejoraría el resultado sin añadir proveedores.
- **El libro de la prueba no falló por las fuentes:** el código escaneado fue `2401114882038`, que **no es un ISBN**. Empieza por 2, el prefijo de los códigos internos de tienda (una etiqueta de precio), y la app lo aceptó porque `Isbn::parse` solo comprueba la longitud y el dígito de control, no el prefijo 978/979 (`crates/domain/src/isbn.rs`).

## Equivalente en inglés: Library of Congress

Probada con tres ISBN en inglés (*The Wise Man's Fear*, *The Hobbit*, *Project Hail Mary*): **3 de 3**, con título, autoría, editorial, año y páginas, entre 0,4 y 1,9 s. También busca por título (`dc.title`).

```
http://lx2.loc.gov:210/lcdb?version=1.1&operation=searchRetrieve&recordSchema=marcxml&query=bath.isbn=9780756404734
```

- **Solo responde por HTTP, sin cifrar** (el puerto 210 no acepta HTTPS). Desde el móvil, Android bloquea el tráfico sin cifrar por defecto y las búsquedas viajarían a la vista. Encaja mejor **a través del servidor**.
- El MARC trae la puntuación de catalogación al final de los campos («The wise man's fear /», «Rothfuss, Patrick,»): hay que limpiarla, para la BNE también.
- La British Library no se ha probado: su catálogo quedó muy limitado tras el ciberataque de 2023.

## Propuesta de orden (por decidir)

Para libros, consultar primero las bibliotecas nacionales según el idioma de la app (BNE en español, Library of Congress en inglés), después Open Library e Inventaire, y Google Books al final, solo con clave.
