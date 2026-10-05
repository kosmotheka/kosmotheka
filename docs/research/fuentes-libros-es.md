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
- **Pendiente:** saber qué ISBN se escaneó en la prueba, porque Open Library e Inventaire sí tienen dos ediciones de ese libro. Si no lo encontró, puede ser otra edición o un fallo nuestro.
