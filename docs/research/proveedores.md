# Proveedores de datos por tipo de catálogo

> Investigación del 25/09/2026, para la [revisión del alcance](../historico/revision-alcance.md). Lo acordado está en D62. Las condiciones de uso cambian: **hay que volver a comprobarlas antes de integrar cada proveedor**.

## Principios (D18, reafirmados)

- **Los proveedores ayudan; nunca son imprescindibles.** Siempre se puede dar de alta cualquier cosa a mano, sin buscar nada. Buscar es un botón de ayuda que rellena el formulario, no un paso obligatorio.
- **Nada se guarda sin que el usuario pulse «Guardar»**, y todo lo que rellena un proveedor se puede corregir.
- **Cada tipo de catálogo tiene sus proveedores**, detrás de una interfaz intercambiable, con dobles en los tests.
- **Buscar por título** (y por autor, año, plataforma…) es tan importante como buscar por código: muchas cosas no tienen código (PDF, ebooks, digitales, una caja perdida).
- El catálogo de tipos es **cerrado**: el usuario no crea tipos nuevos, así que cada proveedor se puede integrar y probar a conciencia.

## Proveedores candidatos

| Tipo | Proveedor | Busca por | Clave | Condiciones | Notas |
|---|---|---|---|---|---|
| **Libros** | [Biblioteca Nacional de España](https://bne.alma.exlibrisgroup.com/) (SRU del catálogo Alma) | ISBN, título | No | Datos del catálogo público | Sin clave; SRU del catálogo Alma; integrado el 05/10/2026. Primer proveedor de libros: manda en ediciones españolas; sin portadas |
| | [Library of Congress](https://www.loc.gov/z3950/lcserver.html) (SRU del catálogo) | ISBN, título | No | Sin usuario ni clave; piden no pasar de 10 peticiones por minuto por programa | Integrado el 13/10/2026, desde la app y cifrado. La mejor para ediciones en inglés; sin portadas. [Notas](fuentes-libros-es.md) |
| | [Open Library](https://openlibrary.org/developers/api) | ISBN, título, autor | No | Datos abiertos | Integrado (D26). Huecos en ediciones españolas |
| | [Google Books](https://developers.google.com/books/docs/v1/using) | ISBN, título, autor | Sí, en la práctica | ~1.000 peticiones al día con clave; sin clave comparte una cuota global que se agota | Integrado (D26), con clave opcional. Mejor en español |
| | [Inventaire](https://data.inventaire.io/) | ISBN, título | No | Datos CC0, federado con Wikidata | Consulta las bibliotecas nacionales británica, francesa y **española**. Candidato para ediciones españolas |
| **Juegos de mesa** | [BoardGameGeek](https://boardgamegeek.com/using_the_xml_api) (API XML) | Título | **Token obligatorio desde julio de 2025**, previa solicitud | [Prohibido el uso comercial](https://boardgamegeek.com/wiki/page/XML_API_Terms_of_Use) sin licencia | La referencia; también tiene expansiones y versiones |
| | [GameUPC](https://gameupc.com) | Código de barras → id de BGG | **Sí** (cabecera `x-api-key`; comprobado el 29/09/2026, su documentación publica una clave de pruebas) | Gratuito, API estable (v1) | Correspondencias recopiladas por la comunidad; no es exhaustivo |
| **Películas y series** | [TMDB](https://developer.themoviedb.org/docs/faq) | Título, año | Sí (gratuita) | [No comercial](https://www.themoviedb.org/api-terms-of-use); exige logo y aviso de atribución; no guardar datos más de 6 meses | La habitual en apps libres. Sin búsqueda por código de barras |
| **Videojuegos** | [IGDB](https://api-docs.igdb.com/) (de Twitch) | Título, plataforma | Sí: credenciales de Twitch (id y secreto) | No comercial gratis; comercial con acuerdo | Muy completa; plataformas y ediciones |
| | [RAWG](https://rawg.io/apidocs) | Título, plataforma | Sí (gratuita) | [Gratis con atribución y enlace](https://rawg.io/tos_api), también comercial hasta 20.000 peticiones al mes | Alternativa a IGDB |
| **Cómics** | [Comic Vine](https://comicvine.gamespot.com/api/) | Título, serie, número | Sí (gratuita) | Estrictamente no comercial; 200 peticiones por recurso y hora; exige caché y enlace | Series y números: encaja con las «series» |
| **Música** | [MusicBrainz](https://musicbrainz.org/doc/MusicBrainz_API) | Código de barras, título, artista | No; exige un User-Agent que identifique la app | Datos principales CC0; [1 petición por segundo por IP](https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting) | La referencia abierta. Distingue la obra (el disco) de cada edición (vinilo, CD, reedición), como obra y edición en los libros |
| | [Cover Art Archive](https://coverartarchive.org/) | Id de MusicBrainz | No | Portadas con sus licencias | Las portadas de MusicBrainz |
| | [Discogs](https://www.discogs.com/developers) | Código de barras, título, artista, sello | Sí, token personal gratuito para buscar | 60 peticiones por minuto con token; [uso comercial permitido con límites](https://support.discogs.com/hc/en-us/articles/360009334593-API-Terms-of-Use) (no cobrar por lo que ellos dan gratis) | La mejor para vinilos y ediciones concretas. Candidata para la nube |
| **Todos** | [Wikidata](https://www.wikidata.org/wiki/Wikidata:Data_access) | Título, identificadores | No | CC0 | Cubre libros, películas, series, juegos de mesa, videojuegos y cómics, con menos detalle. **Respaldo universal sin clave** y enlaza con los ids de los demás (BGG, IGDB, TMDB) |

Códigos de barras: el de un libro es su ISBN (EAN-13 con 978 o 979). Los demás son EAN o UPC de producto. Fuera de los libros, los juegos de mesa (GameUPC) y la música (MusicBrainz, Discogs) tienen servicios gratuitos de código a ficha; para películas y videojuegos, los servicios de productos genéricos son de pago o limitados, así que se busca por título.

## Modelo acordado (en debate, 25/09/2026)

- **Todos los proveedores que se pueda:** el proyecto es gratuito y de código abierto.
- **Las claves van en el servidor.** Quien se descargue el código y monte su servidor pone allí sus propios tokens.
- **App sin nube:** solo funcionan los proveedores sin clave (Open Library, Inventaire, Wikidata, GameUPC, MusicBrainz). La app **avisa** de que hay más disponibles con la nube, con una sección de ayuda que lo explica.
- **App conectada a la nube:** se activan automáticamente los proveedores con clave, porque es el servidor quien resuelve la búsqueda.
- **Google Books sin nube:** se mantiene la opción de poner una clave propia en Ajustes (D26), para quien quiera; sin ella, la app avisa de que ese servicio no está disponible.

**Hecho en 7.8 (primera parte, 11/10/2026): Google Books a través del servidor, solo por ISBN.** Con cuenta, un servidor que tiene `GOOGLE_BOOKS_API_KEY` busca el ISBN por la app y guarda una caché sin saber quién preguntó (30 días lo encontrado, 1 día lo no encontrado). Clave propia en Ajustes sigue teniendo preferencia (D83). Las condiciones de Google (la cuota gratuita es de 1.000 llamadas al día; el servidor se limita a 900) y el diseño están en [architecture/cloud.md](../architecture/cloud.md#buscar-libros-a-través-del-servidor-78-d83). Pendiente para el resto: BoardGameGeek, TMDB, IGDB o RAWG y Comic Vine, cada uno con sus condiciones revisadas antes.

**Hecho en 7.8 (segunda parte, 11/10/2026): TMDB (películas y series) por título a través del servidor.** Con cuenta, un servidor con `TMDB_API_TOKEN` busca por título (hasta 8 candidatos con su detalle: directores o creadores, duración o temporadas, cartel). Condiciones releídas el 11/10/2026 (términos del 20/10/2023): uso no comercial; no guardar sus datos más de 6 meses (la caché guarda 30 días como mucho); atribución con su logo y su aviso, que la app muestra en Ajustes › Proveedores de datos › «Fuentes de datos». Diseño en [architecture/cloud.md](../architecture/cloud.md#buscar-películas-y-series-por-título-en-tmdb-78-segunda-parte-d62). Pendiente: clave propia de TMDB en Ajustes, y BoardGameGeek, IGDB o RAWG y Comic Vine.

**Hecho en 7.8 (tercera parte, 11/10/2026): RAWG (videojuegos) por título a través del servidor.** Con cuenta, un servidor con `RAWG_API_KEY` busca por título (hasta 8 candidatos; el detalle —desarrolladores y editores— de los tres primeros). Condiciones releídas el 11/10/2026 (términos del 01/03/2021): gratis hasta 20.000 peticiones al mes (el servidor se pone 600 al día); atribución con enlace activo a RAWG en cada página donde se usen sus datos, que la app cumple en «Fuentes de datos» y en la pantalla de resultados. RAWG solo admite la clave en la dirección de la llamada, lo que obliga a un cuidado especial con excepciones y log. Diseño en [architecture/cloud.md](../architecture/cloud.md#buscar-videojuegos-por-título-en-rawg-78-tercera-parte-d62). Pendiente: clave propia en Ajustes, y BoardGameGeek, IGDB y Comic Vine.

**Hecho en 7.8 (cuarta parte, 11/10/2026): clave propia para TMDB y RAWG** (D62: todo proveedor con clave la admite en Ajustes). Con ella la app llama directamente al proveedor, con preferencia sobre el servidor de la cuenta, y con las mismas comprobaciones que si la respuesta viniera de un servidor. Ver [architecture/cloud.md](../architecture/cloud.md#clave-propia-para-tmdb-y-rawg-78-cuarta-parte-d62).

**Hecho en 7.8 (quinta parte, 12/10/2026): Comic Vine (cómics) por título, por el servidor y con clave propia.** Series y números, con editorial, guion y dibujo (de los tres primeros). Condiciones releídas el 12/10/2026: estrictamente no comercial; 200 peticiones por recurso y hora (el servidor se queda en 150 por recurso); enlace de vuelta en cada página donde se usen sus datos (en «Créditos» y en los resultados); no un producto que compita con ellos, ni redistribuir sus datos alterados. Comic Vine solo admite la clave en la dirección. Diseño y qué se leyó y qué se supuso de su documentación en [architecture/cloud.md](../architecture/cloud.md#buscar-cómics-por-título-en-comic-vine-78-quinta-parte-d62). Pendiente: BoardGameGeek, que de momento no contesta.

## Lo que esto implica para la arquitectura

- **Claves en una app de código abierto.** Una clave no puede ir dentro de la app: el código es público y cualquiera la extraería (D03, D21). Tres salidas:
  1. **El usuario pone la suya** en Ajustes, como ya se hace con Google Books.
  2. **La nube hace de intermediario:** con cuenta, la app pregunta al servidor y el servidor usa la clave del proyecto. Es otra ventaja de registrarse.
  3. **Proveedores sin clave** (Open Library, Inventaire, Wikidata, GameUPC, MusicBrainz) funcionan siempre, para todos.
- **Uso comercial.** BoardGameGeek, TMDB, IGDB y Comic Vine son gratis solo para uso no comercial. La app libre y gratuita encaja; **un servicio de pago (escenario 8) necesitaría licencias** o limitarse a los proveedores abiertos. Hay que tenerlo presente antes de cobrar nada.
- **Atribución.** TMDB y RAWG exigen mostrar su logo o un enlace allí donde se usan sus datos: la ficha tiene que tener sitio para ello.
- **Caducidad.** TMDB no permite guardar sus datos más de seis meses. Lo que el usuario guarda en su ficha es suyo; lo que haya que vigilar es la copia en bruto de la respuesta (el campo `extra` de D08).

## Ideas para más adelante

- **Importar desde las plataformas digitales:** Steam tiene una API para leer la biblioteca de un usuario (con su clave o su perfil público). Epic no tiene API pública. Sería una forma rápida de dar de alta todos los videojuegos digitales.
- **IA como complemento de cualquier catálogo** (D20): reconocer una foto (una portada, una caja, un lomo), sacar el título y buscarlo en el proveedor del tipo. Opcional y con la clave del usuario o de la nube. Queda lejos.

## Juegos de rol: DriveThruRPG y RPGGeek (mirado el 12/10/2026)

**DriveThruRPG no sirve como proveedor, tal como está.**
- **No tiene una API pública de catálogo.** Lo único que da de forma oficial es una clave de aplicación en los ajustes de la cuenta, y lo que se sabe que devuelve es la biblioteca de compras de esa persona, no la tienda.
- Hay herramientas de terceros (un plugin de metadatos para Calibre, `drpg`) que usan llamadas internas de su web. La documentación que había desapareció, y sus propios autores avisan de que puede romperse sin aviso.
- A los afiliados les da RSS, banners y un widget de más vendidos, no un catálogo para buscar.
- **Sus condiciones de uso no se han podido leer** (la página rechaza la lectura automática). Antes de plantear nada con ellos hay que leerlas enteras en su web y pedir permiso por escrito, como con Comic Vine.
- Además, es una tienda de PDF y de impresión bajo demanda, casi toda en inglés: las ediciones en papel en español no están.

**RPGGeek es la vía que encaja.** Es de BoardGameGeek y usa la misma API y el mismo token que ya está pedido (11/10/2026): los tipos `rpgitem` y `rpgissue` (revistas) se buscan igual que `boardgame`. Sin comprobar todavía: cuánto cubre de las ediciones en español, y si las condiciones del token valen también para rol. Se mira cuando llegue el token, junto con los juegos de mesa.

**En español no se conoce ningún catálogo de rol con API.** Lo que sí hay: los libros de rol publicados en España llevan ISBN (Devir, La Factoría, Nosolorol), así que el código de barras y la BNE ya los encuentran en parte. Los módulos finos y las revistas se quedan fuera.

**Distribuidoras españolas (mirado el 12/10/2026): nada que se pueda usar.** Distribuciones Sombra (dbsombra.com) tiene el catálogo a la vista en su web, con portada y descripción, pero sin ISBN visible, sin descargas, sin API y sin condiciones de uso de los datos. Es un escaparate para tiendas. Leerlo sería copiar su web sin permiso, y no se plantea.

**Acordado el 12/10/2026:** RPGGeek se añade, junto con los juegos de mesa de BoardGameGeek, cuando llegue el token. Para el rol en español, la BNE es lo que hay.

**BoardGameGeek: «Using the XML API» leído entero el 14/10/2026** (versión del 02/07/2025, texto pegado por el maintainer; la web bloquea la lectura automática). Falta leer las condiciones de uso, que es otra página.
- **Registro y token obligatorios** (cabecera `Authorization: Bearer …`, siempre a `boardgamegeek.com`, sin `www`). Pueden tardar una semana o más en contestar.
- **Piden las llamadas desde un servidor y con caché.** Si llegan directamente desde las apps puede ser motivo para suspender la licencia. Encaja con hacerlo por el servidor.
- **No piden token a la gente:** «si tu aplicación es para usuarios finales (no programadores), obtén tu propio token […] no deberías pedir a usuarios finales que no programan que consigan su propio token». **Choca con D62**, que da clave propia a todos los proveedores con clave. **Decidido el 14/10/2026 (D62):** sin clave propia en la app para BoardGameGeek, solo por el servidor y solo con cuenta. Quien monte su servidor registra su propia aplicación y pone su token, que es lo que piden («cada aplicación con su propio token»).
- **Prohibidos los servicios que dan sus datos a otras aplicaciones.** El servidor solo atiende a la app; no se puede abrir para terceros.
- **Atribución obligatoria en apps públicas:** el logo «Powered by BGG», con enlace a BoardGameGeek, a un tamaño en que se lea. Los ficheros del logo los dan ellos; hay que descargarlos.
- **Uso comercial:** es comercial si hay ánimo de lucro, publicidad, cualquier ventaja a cambio de pago, **o donativos**, aunque no den nada a cambio. Con donativos haría falta licencia comercial, «muy probablemente gratis». Hoy no aplica; si algún día se aceptan donativos, hay que pedirla antes.
- **Pueden negar o retirar la licencia** a lo que compita con BGG o le perjudique, y cambiar las reglas cuando quieran. Sin soporte técnico.
- Límites de uso todavía sin cifras; se consultan en «Usage» de la aplicación registrada.

**BoardGameGeek: «XML API Terms of Use» leído entero el 14/10/2026** (texto pegado por el maintainer). Remite además a las condiciones generales del sitio (`boardgamegeek.com/terms`), que no se han leído.
- **Licencia gratuita para reproducir y mostrar los datos, solo con fines estrictamente no comerciales.** La pueden retirar a cualquiera, cuando quieran y sin dar motivo.
- **«No puedes modificar los datos […] de ninguna forma».** Es la condición que más afecta al diseño:
  - El servidor no recorta ni retoca ningún valor de BGG: si un valor no cabe en lo que la app acepta, se descarta entero, no se corta. Elegir qué campos se usan no es modificar; cambiar un valor sí.
  - No se usan sus descripciones: muchas vienen de Wikipedia con licencia CC BY-NC-SA, y recortarlas sería modificarlas.
  - Lo que la persona cambia después en su propio catálogo es su ficha, no los datos de BGG que se muestran como tales. Es una lectura nuestra, no algo que digan.
- **Siempre hay que nombrar a BoardGameGeek como fuente**, y en lo público, el logo «Powered by BGG» con enlace, legible.
- **Prohibido usar sus datos para entrenar IA.** No aplica a nada de lo previsto.
- **No dicen cuánto se pueden guardar los datos.** Una caché de 30 días como la de TMDB no contradice nada de lo leído.
- Pueden cambiar la API y las condiciones cuando quieran, avisando en esa misma página.

**BoardGameGeek: condiciones generales del sitio (`boardgamegeek.com/terms`), leídas hasta el apartado 15.F el 14/10/2026** (texto pegado por el maintainer en dos partes). Lo que afecta:
- **6.B.iv: no usar sus sitios «para entrenar o usar de otro modo como datos para» una IA.** Es más amplio que lo de la API: no solo entrenar, sino usar como datos. **Regla para la foto de la estantería (D19, D20):** los datos de BoardGameGeek nunca se pasan a un modelo de IA, ni para comparar títulos leídos ni para nada. Si la foto se compara con un catálogo, el paso de IA solo lee la foto, y lo de BGG se busca después, sin IA.
- **2:** BoardGameGeek dice no tener relación con las apps hechas sobre sus sitios. La app no puede dar a entender lo contrario: el logo se usa solo como «Powered by BGG», como piden.
- **6.B.i:** nada de robots que pidan más de lo que pediría una persona, «salvo que BGG lo permita expresamente»; la API registrada es ese permiso, dentro de sus límites.
- Lo demás leído (cuentas, contenido subido, privacidad de datos de otros usuarios) no aplica: la app no lee perfiles ni colecciones de nadie.
- **6.D (iv): pueden prohibir cualquier uso que, a su juicio, compita con BGG o desplace su mercado.** Kosmotheka lleva colecciones, algo que BGG también hace. Riesgo bajo, porque la licencia se concede describiendo la app, pero es el motivo por el que pueden retirarla sin más. Por eso BGG es siempre una fuente opcional, nunca la única de los juegos de mesa (Wikidata y GameUPC siguen).
- **6.E:** las imágenes de BGG son contenido subido por sus usuarios. Usarlas sin fines comerciales, por la API, es lo que permite su licencia. Guardar la portada en el dispositivo (D27) no cambia eso.
- **6.F: nada de enmarcar sus páginas ni de usar su nombre en metaetiquetas o texto oculto** sin permiso. Aplica a la landing y a la documentación: BoardGameGeek se nombra donde se usa, nunca como palabra clave escondida.
- 7 (programas maliciosos), 8 a 15 (relación entre las partes, cuentas, ley de EE. UU., DMCA, garantías, responsabilidad, arbitraje en Texas): nada que cambie el diseño.
- Leído hasta el 15.F; del 16 al 21 (incluida la parte de marcas) quedó sin pegar. Las reglas del logo ya las dan las condiciones de la API.

**Logo «Powered by BGG»:** hay una captura pegada por el maintainer fuera del repositorio (`library-privado/bgg/`). Para la app hacen falta los ficheros oficiales que enlaza su página.

**Dónde sale el rol (propuesta del maintainer, 14/10/2026):** el rol es un nicho dentro de los libros, y la app es para todo el mundo. Por eso **RPGGeek solo se consulta al buscar libros por título, nunca por ISBN** (su API tampoco busca por ISBN). Si encuentra algo, sale; si no, no se nota. Apuntes de la sesión coordinadora, por confirmar al construirlo:
- Sus resultados van **al final** de los de libros y con su nombre de fuente, para que un «Dune» no llene la lista de suplementos de rol.
- Solo libros de rol (`rpgitem`); las revistas (`rpgissue`) quedan fuera por ahora.
- Se puede apagar en Proveedores, como cualquier fuente.
- Cada búsqueda de libros por título gasta una llamada a BoardGameGeek: cuenta para sus límites y los del servidor.
- Sin tipo «rol» nuevo en la app: lo que se añade desde RPGGeek es un libro.

## Comic Vine: por qué no tiene clave en el servidor oficial

Decidido el 12/10/2026 ([D62](../DECISIONS.md)). La página de su API pide no construir «some crappy collection app» ni un producto que compita con ellos. Por eso el servidor oficial no pone clave y en la app funciona solo con la clave propia de cada persona. El código del servidor está hecho y probado: basta poner `COMIC_VINE_API_KEY` el día que haya permiso.

### Correo a Comic Vine (borrador)

Sin enviar. Lo manda el maintainer cuando quiera, desde el correo del proyecto, al contacto que indique su página de la API o su foro de desarrolladores.

> **Subject:** Using the Comic Vine API in a free, open-source household catalogue app
>
> Hello,
>
> I maintain Kosmotheka (https://kosmotheka.com), a free, open-source (AGPL-3.0), non-commercial app that a household uses to keep a catalogue of what it owns: books, comics, board games, films and games. No ads, no paid tiers, no sale of data.
>
> I have read your API terms, including "don't just build a collection app" and "don't build a competing product", so I would rather ask than assume.
>
> What we would do with the API: when a person adds a comic to their own catalogue, they search by title and pick a result to prefill the entry (series or issue name, number, year, publisher, cover, writer and artist). Nothing else: we do not mirror the database, show wiki content, or offer your data as a product. Every screen that shows your data links back to Comic Vine.
>
> Today this only works if each person enters their own API key. My question is whether you would also allow our server to make those same searches with one project key on behalf of signed-in users. The server caches answers (searches for 7 days, details for 30), stays under 150 requests per resource per hour, and never makes bulk or background requests.
>
> If the answer is no, we will keep it to personal keys, or remove it if you prefer. Thank you for the API either way.
>
> Sergio Rebollo
> info@kosmotheka.com

## GameUPC: lo que hay y lo que falta (leído en su página el 12/10/2026)

**Qué es.** Un servicio de una sola persona, alimentado por la comunidad: dado el código de barras de una caja, dice qué juego de BoardGameGeek es (nombre, identificador de BGG y sus imágenes). Si no está seguro, devuelve varios candidatos para que la persona elija, y admite que la app le devuelva la elección para verificarla (unos 15.000 códigos verificados).

**Cómo está hoy en la app** (Fase 6): solo funciona con la **clave propia** de quien la ponga en Proveedores; sin clave no se le llama. No hay ruta en el servidor: con cuenta no «se activa solo», al contrario que los demás (D62).

**Lo leído** en `gameupc.com` y en su descripción de la API (`gameupc-oas.yaml`):

- Dos entornos: `test`, con una clave pública de pruebas, y `v1`, «Production database; **email for API key**». La clave va en la cabecera `x-api-key`.
- **No hay página de condiciones**: ni uso comercial, ni atribución, ni caché. Lo único escrito es un límite: **100 códigos nuevos al día** (nuevos = que su base aún no conoce; responde «Overloaded; try again later»).
- Contacto: el nombre del sitio en `@grettir.org`.
- Las imágenes son las de BoardGameGeek (`cf.geekdo-images.com`), que la app ya acepta.

**Lo que no sabemos y hay que preguntar en el mismo correo en que se pide la clave:** si le parece bien que un servidor busque con una clave del proyecto en nombre de sus cuentas, cuánto se pueden guardar las respuestas y si quiere algún crédito.

**Plan** (maintainer, 12/10/2026): la ruta del servidor para GameUPC se hace **junto con BoardGameGeek**, cuando BGG conceda el token: GameUPC traduce el código y BGG da la ficha; por separado, por el servidor solo aportaría el nombre.

**Respuesta de GameUPC (15/10/2026, pegada por el maintainer; la clave de producción ya la tiene él, fuera del repositorio):**
- **Le parece bien que el servidor llame con la clave del proyecto en nombre de sus cuentas.** Solo dan claves a quien hace la app, nunca a usuarios: no tienen el concepto de que cada persona tenga la suya.
- **Lo que pide:** que en cada escritura (devolverle el id de BGG elegido) vaya un identificador del usuario, para poder deshacer lo que escriba alguien de mala fe sin tirar todo lo de nuestra clave. Dice que nunca ha tenido que hacerlo.
  - Propuesta de la sesión coordinadora, por confirmar: un identificador seudónimo y estable por cuenta, sacado en el servidor con una clave secreta (como la huella del correo), nunca el id real de la cuenta ni el correo. Sirve para lo que pide y no les dice quién es nadie. Sin cuenta no se escribe.
- **Tres caminos que pide implementar:** código validado (una respuesta segura); código con sugerencias (la persona elige); código sin datos (la persona busca el juego). En los dos últimos, **devolver el id de BGG elegido** a `/upc/{upc}/bgg_id/{bgg_id}`: el 20 % de los códigos son nuevos y de eso vive su base.
- Códigos de prueba: `111111111117` (respuesta segura), `222222222224` (sugerencias), `333333333331` (sin datos), y un PDF con sus códigos de barras en su web.
- Sin condiciones de caché ni de atribución, igual que antes.
- **Choca con D62 (clave propia para todos)** igual que BoardGameGeek: él solo contempla la clave del proyecto. A decidir al construirlo junto con BGG.

### Correo a GameUPC (borrador)

Enviado por el maintainer el 12/10/2026, tal cual. A la espera de respuesta.

> **Subject:** API key for Kosmotheka, a free open-source household catalogue app
>
> Hello,
>
> I maintain Kosmotheka (https://kosmotheka.com), a free, open-source (AGPL-3.0), non-commercial app for Android, Windows and Linux that a household uses to catalogue what it owns, board games included. When someone scans the barcode on a game box, we would like to ask GameUPC which game it is and prefill their entry.
>
> Could I have a production API key? And three questions, since I found no terms page and would rather ask than assume:
>
> 1. Is it fine if our own server makes the lookups with one project key on behalf of signed-in users? The key would stay on the server, never inside the app. People without an account would enter their own key.
> 2. May the server cache answers (we use 30 days for a found barcode, 1 day for an unknown one)? It keeps traffic to you low.
> 3. Would you like a credit or a link in the app? We already have a credits section for data sources.
>
> Volume is very low (a handful of users in closed testing). When a person picks the right game among several suggestions, we can also post that choice back to you so it gets verified.
>
> Thank you for running the service.
>
> Sergio Rebollo
> info@kosmotheka.com

