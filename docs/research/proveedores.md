# Proveedores de datos por tipo de catálogo

> Investigación del 25/09/2026, para la [revisión del alcance](../plans/revision-alcance.md). Amplía D21. Las condiciones de uso cambian: **hay que volver a comprobarlas antes de integrar cada proveedor**.

## Principios (D18, reafirmados)

- **Los proveedores ayudan; nunca son imprescindibles.** Siempre se puede dar de alta cualquier cosa a mano, sin buscar nada. Buscar es un botón de ayuda que rellena el formulario, no un paso obligatorio.
- **Nada se guarda sin que el usuario pulse «Guardar»**, y todo lo que rellena un proveedor se puede corregir.
- **Cada tipo de catálogo tiene sus proveedores**, detrás de una interfaz intercambiable, con dobles en los tests.
- **Buscar por título** (y por autor, año, plataforma…) es tan importante como buscar por código: muchas cosas no tienen código (PDF, ebooks, digitales, una caja perdida).
- El catálogo de tipos es **cerrado**: el usuario no crea tipos nuevos, así que cada proveedor se puede integrar y probar a conciencia.

## Proveedores candidatos

| Tipo | Proveedor | Busca por | Clave | Condiciones | Notas |
|---|---|---|---|---|---|
| **Libros** | [Open Library](https://openlibrary.org/developers/api) | ISBN, título, autor | No | Datos abiertos | Integrado (D26). Huecos en ediciones españolas |
| | [Google Books](https://developers.google.com/books/docs/v1/using) | ISBN, título, autor | Sí, en la práctica | ~1.000 peticiones al día con clave; sin clave comparte una cuota global que se agota | Integrado (D26), con clave opcional. Mejor en español |
| | [Inventaire](https://data.inventaire.io/) | ISBN, título | No | Datos CC0, federado con Wikidata | Consulta las bibliotecas nacionales británica, francesa y **española**. Candidato para ediciones españolas |
| **Juegos de mesa** | [BoardGameGeek](https://boardgamegeek.com/using_the_xml_api) (API XML) | Título | **Token obligatorio desde julio de 2025**, previa solicitud | [Prohibido el uso comercial](https://boardgamegeek.com/wiki/page/XML_API_Terms_of_Use) sin licencia | La referencia; también tiene expansiones y versiones |
| | [GameUPC](https://gameupc.com) | Código de barras → id de BGG | No | Gratuito, API estable (v1) | Correspondencias recopiladas por la comunidad; no es exhaustivo |
| **Películas y series** | [TMDB](https://developer.themoviedb.org/docs/faq) | Título, año | Sí (gratuita) | [No comercial](https://www.themoviedb.org/api-terms-of-use); exige logo y aviso de atribución; no guardar datos más de 6 meses | La habitual en apps libres. Sin búsqueda por código de barras |
| **Videojuegos** | [IGDB](https://api-docs.igdb.com/) (de Twitch) | Título, plataforma | Sí: credenciales de Twitch (id y secreto) | No comercial gratis; comercial con acuerdo | Muy completa; plataformas y ediciones |
| | [RAWG](https://rawg.io/apidocs) | Título, plataforma | Sí (gratuita) | [Gratis con atribución y enlace](https://rawg.io/tos_api), también comercial hasta 20.000 peticiones al mes | Alternativa a IGDB |
| **Cómics** | [Comic Vine](https://comicvine.gamespot.com/api/) | Título, serie, número | Sí (gratuita) | Estrictamente no comercial; 200 peticiones por recurso y hora; exige caché y enlace | Series y números: encaja con las «series» |
| **Todos** | [Wikidata](https://www.wikidata.org/wiki/Wikidata:Data_access) | Título, identificadores | No | CC0 | Cubre libros, películas, series, juegos de mesa, videojuegos y cómics, con menos detalle. **Respaldo universal sin clave** y enlaza con los ids de los demás (BGG, IGDB, TMDB) |

Códigos de barras: el de un libro es su ISBN (EAN-13 con 978 o 979). Los demás son EAN o UPC de producto. Fuera de los libros, solo los juegos de mesa tienen un servicio gratuito de código a ficha (GameUPC); para películas y videojuegos, los servicios de productos genéricos son de pago o limitados, así que se busca por título.

## Modelo acordado (en debate, 25/09/2026)

- **Todos los proveedores que se pueda:** el proyecto es gratuito y de código abierto.
- **Las claves van en el servidor.** Quien se descargue el código y monte su servidor pone allí sus propios tokens.
- **App sin nube:** solo funcionan los proveedores sin clave (Open Library, Inventaire, Wikidata, GameUPC). La app **avisa** de que hay más disponibles con la nube, con una sección de ayuda que lo explica.
- **App conectada a la nube:** se activan automáticamente los proveedores con clave, porque es el servidor quien resuelve la búsqueda.
- **Google Books sin nube:** se mantiene la opción de poner una clave propia en Ajustes (D26), para quien quiera; sin ella, la app avisa de que ese servicio no está disponible.

## Lo que esto implica para la arquitectura

- **Claves en una app de código abierto.** Una clave no puede ir dentro de la app: el código es público y cualquiera la extraería (D03, D21). Tres salidas:
  1. **El usuario pone la suya** en Ajustes, como ya se hace con Google Books.
  2. **La nube hace de intermediario:** con cuenta, la app pregunta al servidor y el servidor usa la clave del proyecto. Es otra ventaja de registrarse.
  3. **Proveedores sin clave** (Open Library, Inventaire, Wikidata, GameUPC) funcionan siempre, para todos.
- **Uso comercial.** BoardGameGeek, TMDB, IGDB y Comic Vine son gratis solo para uso no comercial. La app libre y gratuita encaja; **un servicio de pago (escenario 8) necesitaría licencias** o limitarse a los proveedores abiertos. Hay que tenerlo presente antes de cobrar nada.
- **Atribución.** TMDB y RAWG exigen mostrar su logo o un enlace allí donde se usan sus datos: la ficha tiene que tener sitio para ello.
- **Caducidad.** TMDB no permite guardar sus datos más de seis meses. Lo que el usuario guarda en su ficha es suyo; lo que haya que vigilar es la copia en bruto de la respuesta (el campo `extra` de D08).

## Ideas para más adelante

- **Importar desde las plataformas digitales:** Steam tiene una API para leer la biblioteca de un usuario (con su clave o su perfil público). Epic no tiene API pública. Sería una forma rápida de dar de alta todos los videojuegos digitales.
- **IA como complemento de cualquier catálogo** (D20): reconocer una foto (una portada, una caja, un lomo), sacar el título y buscarlo en el proveedor del tipo. Opcional y con la clave del usuario o de la nube. Queda lejos.
