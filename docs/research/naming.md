# Nombre del proyecto

> Abierto. «library» es provisional. No bloquea el desarrollo: carpetas, *crates* e identificadores se pueden renombrar. El identificador de Android solo importa de verdad al publicar en una tienda. Al cambiarlo, las APK de prueba se desinstalan y se vuelven a instalar (y los datos locales del móvil se recuperan sincronizando).

## Criterios

- **Global, no solo para hispanohablantes** (26/09/2026). La app sale en inglés y español; al ser open source, podrá conectarse a un servicio de traducción colaborativa para añadir idiomas. Los que se escriben de derecha a izquierda (árabe) se verán cuando lleguen.
- Corto y fácil de pronunciar en español y en inglés.
- Que funcione escrito en un enlace o un mensaje: se comparte con gente que no tiene la app.
- **No limitado a los libros**: también habrá juegos, cómics, música y películas.
- Libre: repo en GitHub, dominio, Play Store y marca registrada.

**Por decidir:** hay dos enfoques que tiran en direcciones contrarias.
- **Un nombre que no signifique nada evidente** (Funes): no encasilla. Es la idea de la primera ronda (24/09/2026).
- **Un nombre que diga lo que hace** (estante, balda, lomo, tejuelo, «-teca»): se entiende a la primera, pero casi todos suenan a libros y son difíciles en inglés. Es la idea de la segunda ronda (26/09/2026).

## Candidatos

### Primera ronda (24/09/2026)

| Nombre | Idea | Pega | Opinión del maintainer |
|---|---|---|---|
| **Funes** | «Funes el memorioso» (Borges): el que lo recuerda todo | Guiño que no todo el mundo pilla | Le gusta: al no significar nada evidente, no encasilla |
| **Kinshelf** | *Kin* (familia) + *shelf* (estantería) | Suena solo a libros | Le gusta más, pero duda por eso |
| Balda | Donde viven libros y juegos | Muy español; balda.app está cogido | — |
| Holding | *Holdings* de biblioteca + la *Bag of Holding* de D&D | En inglés también es «sociedad holding» | — |
| Tomo | Volumen; igual en español, inglés y japonés | Suena a libros y está muy usado | — |
| Lomo | El lomo del libro, lo que lee la foto de la balda | En España, embutido; lomo.app está cogido | — |

Ninguno de estos se ha pasado todavía por TMview ni por los registros de dominios.

### Segunda ronda (26/09/2026)

Dominios consultados el 26/09/2026 en los registros oficiales (Verisign para .com, Google Registry para .app). Marcas buscadas en TMview el mismo día con [`tmview-check.mjs`](tmview-check.mjs): búsqueda «Contiene» en todas las oficinas. Capturas y datos en [`tmview/`](tmview/).

| Nombre | Qué evoca | .com | .app | Marcas en TMview | Conflicto (clases 9, 41, 42) |
|---|---|---|---|---|---|
| ★ **Estanteca** | Estante + «-teca»; sirve para libros y juegos | Libre | Libre | Ninguna | No |
| ★ **Coleteca** | Colección + «-teca»; ideal si hay varios catálogos | Libre | Libre | Ninguna | No |
| ★ **Tejuelo** | La etiqueta del lomo en las bibliotecas | Cogido (agosto 2026); tejueloapp.com libre | Libre | 5 (ver abajo) | **Sí**: 41 en Argentina |
| **Anaquelia** | De «anaquel», la balda de librería | Libre | Sin comprobar | Ninguna | No |
| **Baldario** | Balda + «-ario», como herbario | Libre | Libre | 1: Egipto, clase 5 (farmacia) | No |
| **Lomoteca** | Biblioteca de lomos | Libre | Sin comprobar | 5, todas de restauración | No |
| **Mibalda** | «Mi balda» | Libre | Sin comprobar | 2 en Corea, por transliteración | No |
| **Ludobalda** | Posible marca para la ludoteca | Libre | Sin comprobar | Ninguna | No |

Otras apps con nombres parecidos: estante.app es de Estante Digital (Brasil); hay apps llamadas «Coleccionista» y «MisColecciones+».

### Marcas encontradas

**Tejuelo**
| Marca | Titular | Oficina | Estado | Clases |
|---|---|---|---|---|
| TEJUELO EDITORES (2021) | una persona física (titular) | Argentina (INPI) | Registrada | **41** (publicación de libros, también electrónica) |
| TEJUELO EDITORES (2010) | una persona física (titular) | Argentina (INPI) | Registrada | **41** |
| TEJUELOS (con logotipo) | una persona física (titular) | Perú (INDECOPI) | Solicitada | 43 |
| MONTEJUELO | una persona física (titular) | España (OEPM) | Registrada | 33 |
| NAVA DEL TEJUELO | Quintaplast La Mancha, S.L. | España (OEPM) | Solicitada | 33 |

**Baldario**: BALDARIO, de Shandong Sino-Agri United Biotechnology Co. Egipto, registrada, clase 5.

**Lomoteca**
| Marca | Titular | Oficina | Estado | Clases |
|---|---|---|---|---|
| LOMOTECA LOS CAROLINOS | una persona física (titular) | Argentina | Vencida | 35 |
| LA LOMOTECA | una persona física (titular) | Argentina | Vencida | 43 |
| PARRILLA A LA CHAPA LOMOTECA | A la Chapa SAS | Argentina | Registrada | 35 y 43 (dos marcas) |
| LA LOMOTECA ASADOR ARGENTINO… | dos personas físicas (titulares) | España (OEPM) | Registrada | 43 |

**Mibalda**: 미발단 y 봄 미발단, de 장영철. Corea del Sur, finalizadas, clase 30.

Límites: la búsqueda «Contiene» solo encuentra el texto literal, no marcas que se parezcan por sonido o forma. TMview no es un registro oficial.

### Tercera ronda (26/09/2026): colección general y global

Enfoque elegido: evocar **colección en general** (juegos, música, lo digital), sin muebles ni libros, y que funcione en inglés y en español. Dominios por RDAP (Verisign para .com, Google para .app); marcas en TMview con el script, fijándose solo en las marcas iguales al nombre.

**Palabras reales: todas cogidas.** Vitrina, vitrine, arca, kist, omnia, collecta, arkive, musea y curiosa tienen .com y .app registrados y entre 13 y 54 marcas en las clases 9, 41 o 42.

**Inventadas: el .com está cogido en todas** (29 probadas, de cinco a nueve letras). Un .com corto y pronunciable es prácticamente imposible; habrá que ir a .app o a una variante (`getX.com`, `Xapp.com`).

| Nombre | Idea | .app | Marcas en TMview | GitHub | Valoración |
|---|---|---|---|---|---|
| **Musary** | *Museum* + «-ary» (como *library*, *aviary*): un museo personal | Libre | Ninguna igual ni parecida | Usuario vacío (0 repos) | El más limpio. El .com es de un holandés desde 2009, sin web |
| **Colleo** | Evoca *collect* | Libre | COLLEO vigente solo en inmobiliaria (36, Alemania); las de la 42 están vencidas | Usuario vacío (0 repos) | Posible. El .com está a la venta |
| Omnika | *Omni*: todo | Libre | «OMNIKA 5.7» registrada en Portugal en 41 y 42 | — | Dudoso |
| Arkeep | El arca (uno de cada) + *keep* | Libre | ARKEEP registrada en la UE en la clase 9 | — | Descartado |

**Descartados por cómo suenan** (con .app libre): Hoardia (*hoarder*, acumulador compulsivo), Cosario (suena a «corsario»), Arkadu, Colectio.

**.app cogido:** kollo, arkeo, vitrino, omnio, tenko, kisto, curato, trovi, shelvo, kepto, arkio, amasso, pilo, stashi, havly, gathero, ownbox, vitrin, collectia, omnikeep, kollecta, arkivo.

### Descartados

- **Marcas de software de bibliotecas:** Ex Libris y Aleph.
- **Demasiado usados:** Babel, Mimir y Trove.
- **Ya son apps de libros o colecciones** (de una lista de ChatGPT): Nook (Barnes & Noble), Trove, Curio, Kollect, Hoardy, Librio y Stash.
- **En uso en otros sectores:** Keevi, Keepio, Kivvi, Nolio, Tovio, Tavilo y Folko.
- **Dominio cogido:** anaquel.com, anaquel.app, tejuelo.com, lomario.com, estantia.com, lotengo.com, yalotengo.com, prestame.com, dejamelo.com, tomario.com, tomoteca.com, prestateca.com, ludoteka.com, balda.app, lomo.app, estante.app, lendshelf.com, shelfmark.com, spinewise.com, spineshelf.com, shelfhold.com, shelfkin.com, tomekeeper.com, tomewise.com, lendkin.com, ownshelf.com y havelist.com.

Varios de esos dominios se registraron en 2026: los nombres buenos vuelan. Cuando haya decisión, conviene registrarlo pronto.

## Siguientes pasos

1. Decidir el enfoque: nombre neutro o nombre que diga lo que hace.
2. Pasar por TMview los nombres de la primera ronda que sigan vivos (Funes, Kinshelf): `node docs/research/tmview-check.mjs funes kinshelf`.
3. Con dos o tres favoritos:
   - Comprobar los .app que faltan y los .es.
   - Buscar por similitud en el buscador de la EUIPO (eSearch plus), no solo por texto literal.
   - Comprobar GitHub y Google Play.
   - Ver cómo aparecen en un buscador.
